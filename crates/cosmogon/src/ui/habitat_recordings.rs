//! Independent recorded observations, never edits or fabricated live history.
use super::{ACCENT, MUTED};
use bevy::{
    prelude::*,
    tasks::{block_on, futures_lite::future, AsyncComputeTaskPool, Task},
};
use bevy_egui::egui;
use cosmogon_sim::{
    habitat_comparison::{self, Alignment},
    habitat_factorial::{Design, Effect, Point},
    habitat_recording::{Metric, Recording},
    habitat_sweep::{self, Parameter, Plan},
    Universe,
};

fn parse_levels(s: &str) -> Result<Vec<f64>,String> {
    s.split(',').map(|s|s.trim().parse::<f64>().map_err(|_|"Enter finite comma-separated values".into())).collect()
}
pub fn check_input(path: &std::path::Path,sample: Option<usize>) -> Result<(),String> {
    let count=if path.join("plan.json").exists() {cosmogon_sim::habitat_design::read(path)?.recordings()[0].rows.len()} else {Recording::read(path)?.rows.len()};
    if sample.is_some_and(|i|i>=count) {return Err("Requested habitat observation is outside the recording".into());}
    Ok(())
}
fn open_design(location:std::path::PathBuf,path:String,sample:Option<usize>) -> Result<SweepResult,String> {
    let design=cosmogon_sim::habitat_design::read(&location)?;
    let a=design.recordings()[0].clone();
    let b=design.recordings().last().ok_or("Empty design")?.clone();
    let manifest=design.manifest().clone();
    let origin=cosmogon_sim::habitat_origin::Origin::from_manifest(&location,&manifest)?;
    let (single,paired)=match design {
        cosmogon_sim::habitat_design::Design::Single(d)=>(Some(d),None),
        cosmogon_sim::habitat_design::Design::Paired(d)=>(None,Some(d)),
    };
    Ok(SweepResult {a,b,manifest,paired,single,origin:Some(origin),origin_selection:None,path,sample})
}
pub struct Launch {
    pub plan: Plan,
    pub id: Option<u64>,
    pub output: String,
    pub sample: Option<usize>,
    pub initial_path: Option<String>,
}
fn run_sweep(
    base: Universe,
    id: u64,
    plan: Plan,
    path: String,
    sample: Option<usize>,
) -> Result<SweepResult, String> {
    habitat_sweep::execute(&base, id, &plan, std::path::Path::new(&path))?;
    open_design(std::path::PathBuf::from(&path),path,sample)
}
struct SweepResult {
    a: Recording,
    b: Recording,
    manifest: serde_json::Value,
    paired: Option<Design>,
    single: Option<cosmogon_sim::habitat_design::Single>,
    origin: Option<cosmogon_sim::habitat_origin::Origin>,
    origin_selection: Option<bool>,
    path: String,
    sample: Option<usize>,
}
struct SweepForm {
    parameter: Parameter,
    values: String,
    samples: usize,
    interval_hours: f64,
    output: String,
    paired: bool,
    secondary_parameter: Parameter,
    secondary_values: String,
    band: String,
}
impl Default for SweepForm {
    fn default() -> Self {
        Self {
            parameter: Parameter::FirmPowerW,
            values: "0, 2000, 4000".into(),
            samples: 25,
            interval_hours: 1.0,
            output: String::new(),
            paired: false,
            secondary_parameter: Parameter::WaterRecoveryFraction,
            secondary_values: "0.8, 0.95".into(),
            band: String::new(),
        }
    }
}

#[derive(Default)]
pub struct Study {
    origin: Option<cosmogon_sim::habitat_origin::Origin>,
    use_saved_origin: bool,
    origin_validation: Option<(Plan,Result<(),String>)>,
    sweep_request: Option<serde_json::Value>,
    pub requested_origin: Option<bool>,
    pub rerun_output: Option<String>,
    pub recording: Option<Recording>,
    pub comparison: Option<Recording>,
    loaded_band: Option<cosmogon_sim::research::RunBand>,
    comparison_band: Option<cosmogon_sim::research::RunBand>,
    pub experiment_mode: bool,
    pub keep_experiment_at_launch: bool,
    pub factorial_mode: bool,
    pub paired_pane: usize,
    pub effect: Effect,
    paired: Option<Design>,
    single: Option<cosmogon_sim::habitat_design::Single>,
    single_source: String,
    branches: [usize;2],
    paired_curve: Option<(Metric, Vec<Point>)>,
    paired_output: String,
    paired_source: String,
    paired_export: Option<Task<Result<(),String>>>,
    pub balance_mode:bool,
    pub balance:super::habitat_balance::View,
    pub figure_mode:bool,
    pub figure:super::habitat_figure::View,
    pub pressure_mode:bool,
    pub pressure:super::habitat_pressure::View,
    pub why_mode: bool,
    pub why: super::habitat_why::View,
    pub replay: super::habitat_replay::View,
    pub graph:super::habitat_graph::View,
    pub graph_mode:bool,
    pub alignment: Alignment,
    comparison_path: String,
    comparison_output: String,
    comparison_task: Option<Task<Result<Recording, String>>>,
    pending_comparison_path: String,
    sweep_task: Option<Task<Result<SweepResult, String>>>,
    sweep: SweepForm,
    pub sweep_manifest: Option<serde_json::Value>,
    pub launch: Option<Launch>,
    pub requested_branches: Option<[usize;2]>,
    pub metric: Metric,
    pub selected: usize,
    path: String,
    loaded_path:String,
    pub status: Option<String>,
    task: Option<Task<Result<Recording, String>>>,
    pending_path: String,
    requested_sample: Option<usize>,
}
impl Study {
    pub fn apply_graph_layout(&mut self) {
        self.metric=Metric::parse(&self.graph.layout.metric).expect("validated layout metric");
        self.effect=Effect::parse(&self.graph.layout.effect).expect("validated layout effect");
        self.alignment=Alignment::parse(&self.graph.layout.alignment).expect("validated layout alignment");
        self.paired_curve=None;
    }
    pub fn sync_graph_layout(&mut self) {
        self.graph.layout.metric=self.metric.id().into();self.graph.layout.effect=self.effect.id().into();self.graph.layout.alignment=match self.alignment {Alignment::Elapsed=>"elapsed",Alignment::World=>"world"}.into();
    }
    fn graph_metadata(&self)->serde_json::Value {
        let Some(r)=&self.recording else{return serde_json::Value::Null;};
        let series=if self.factorial_mode {self.paired.as_ref().and_then(|d|d.curve(self.metric).ok()).map(|p|paired_series(&p,self.effect,self.alignment)).unwrap_or_default()}else{ordinary_series(r,self.comparison.as_ref(),self.alignment,self.metric)};
        let mut layout=self.graph.layout.clone();layout.metric=self.metric.id().into();layout.effect=self.effect.id().into();layout.alignment=match self.alignment{Alignment::Elapsed=>"elapsed",Alignment::World=>"world"}.into();
        self.graph.metadata(&series,self.selected,&layout)
    }
    pub fn start(&mut self, path: String, sample: Option<usize>) {
        let location = std::path::PathBuf::from(&path);
        if location.join("plan.json").exists() {
            self.sweep_task=Some(AsyncComputeTaskPool::get().spawn(async move {open_design(location,path,sample)}));
            self.status=Some("Checking all retained common-state branches and design identity…".into());
            return;
        }
        self.task =
            Some(AsyncComputeTaskPool::get().spawn(async move { Recording::read(&location) }));
        self.pending_path = path;
        self.requested_sample = sample;
        self.status = Some("Checking recorded sources, endpoints and observations…".into());
    }
    pub fn pending(&self) -> bool {
        self.balance.pending() || self.figure.pending() || self.pressure.pending() || self.replay.pending() || self.paired_export.is_some() || self.task.is_some()
            || self.comparison_task.is_some()
            || self.sweep_task.is_some()
            || self.launch.is_some() || self.rerun_output.is_some()
    }
    pub fn start_comparison(&mut self, path: String) {
        let location = std::path::PathBuf::from(&path);
        self.comparison_task =
            Some(AsyncComputeTaskPool::get().spawn(async move { Recording::read(&location) }));
        self.pending_comparison_path = path;
        self.status = Some("Checking comparison recording…".into());
    }
    pub fn start_sweep(
        &mut self,
        base: Universe,
        id: u64,
        plan: Plan,
        path: String,
        sample: Option<usize>,
    ) {
        self.sweep_request=Some(serde_json::json!({"origin":{"kind":"current_live_world","simulation_time_s":base.time,"habitat_id":id,"state_fingerprint":cosmogon_sim::research::state_fingerprint(&base).ok(),"data_versions":cosmogon_sim::save::data_versions(&base)},"plan":plan,"output":path,"scope":"Exact request-time independent starting world; completed observations remain unchanged until success"}));
        self.sweep_task = Some(
            AsyncComputeTaskPool::get()
                .spawn(async move {let mut result=run_sweep(base,id,plan,path,sample)?;result.origin_selection=Some(false);Ok(result)}),
        );
        self.status = Some("Running independent habitat branches; live world is unchanged…".into());
    }
    fn start_saved_sweep(&mut self, plan: Plan, path: String) -> Result<(),String> {
        let origin=self.origin.clone().ok_or("Open a complete design to reuse its original starting world")?;
        let sample=Some(self.selected.min(plan.samples.saturating_sub(1)));
        self.sweep_request=Some(serde_json::json!({"origin":origin.metadata(),"plan":plan,"output":path,"scope":"Retained source pins captured at request; actual common file is rechecked before writing any results"}));
        self.sweep_task=Some(AsyncComputeTaskPool::get().spawn(async move {
            let world=origin.load()?;
            let mut result=run_sweep(world,origin.habitat_id,plan,path,sample)?;
            result.origin_selection=Some(true);Ok(result)
        }));
        self.status=Some("Checking pinned original starting world and running independent branches…".into());
        Ok(())
    }
    fn saved_plan_validation(&mut self, plan: &Plan) -> Result<(),String> {
        let origin=self.origin.as_ref().ok_or("Open a complete design to choose its saved starting world")?;
        if self.origin_validation.as_ref().is_none_or(|(cached,_)|cached!=plan) {
            self.origin_validation=Some((plan.clone(),plan.validate_world(origin.cached_world(),origin.habitat_id)));
        }
        self.origin_validation.as_ref().unwrap().1.clone()
    }
    fn cancel_dependent_launches(&mut self) {
        // A requested new run failed: never export or verify previous observations as its result.
        self.rerun_output=None;self.requested_origin=None;self.requested_branches=None;
        self.figure.launch=false;self.balance.launch=false;self.replay.launch=false;
        self.keep_experiment_at_launch=false;
    }
    pub fn launch_if_ready(&mut self, base: &Universe, selected: Option<u64>, reference: bool) {
        if self.rerun_output.is_some() && self.origin.is_some() && self.task.is_none() && self.sweep_task.is_none() && self.comparison_task.is_none() {
            let output=self.rerun_output.take().unwrap();
            if reference {self.cancel_dependent_launches();self.status=Some("Clone this reference world to a sandbox before running a new experiment".into());return;}
            let plan=match self.sweep_manifest.as_ref().and_then(|m|serde_json::from_value::<Plan>(m["plan"].clone()).ok()){Some(p)=>p,None=>{self.cancel_dependent_launches();self.status=Some("Open a complete design before rerunning".into());return;}};
            if self.single.is_some(){self.requested_branches=Some(self.branches);}
            self.keep_experiment_at_launch=self.experiment_mode;
            if self.use_saved_origin {
                if let Err(e)=self.start_saved_sweep(plan,output){self.cancel_dependent_launches();self.status=Some(e);}
            }else{
                let id=self.origin.as_ref().unwrap().habitat_id;
                let sample=Some(self.selected.min(plan.samples.saturating_sub(1)));
                self.start_sweep(base.clone(),id,plan,output,sample);
            }
            return;
        }
        if let Some(launch) = self.launch.take() {
            if reference {
                self.cancel_dependent_launches();
                self.status =
                    Some("Clone the reference world before running habitat experiments.".into());
                return;
            }
            if let Some(file) = launch.initial_path {
                self.sweep_task = Some(AsyncComputeTaskPool::get().spawn(async move {
                    let base = cosmogon_sim::save::read_file(std::path::Path::new(&file))
                        .map_err(|e| e.to_string())?;
                    let id = launch
                        .id
                        .or_else(|| (base.habitats.len() == 1).then(|| base.habitats[0].id))
                        .ok_or("Choose a habitat ID in the saved common world")?;
                    run_sweep(base, id, launch.plan, launch.output, launch.sample)
                }));
                self.status = Some(
                    "Running from the exact saved common world; live state remains independent…"
                        .into(),
                );
                return;
            }
            let id = launch
                .id
                .or(selected)
                .or_else(|| (base.habitats.len() == 1).then(|| base.habitats[0].id));
            if let Some(id) = id {
                self.start_sweep(base.clone(), id, launch.plan, launch.output, launch.sample);
            } else {
                self.cancel_dependent_launches();
                self.status = Some("Choose a habitat in the common starting world.".into());
            }
        }
    }
    fn commit_sweep(&mut self, mut result: SweepResult) -> Result<(), String> {
        let index = result.sample.unwrap_or(0);
        if index >= result.a.rows.len() {
            return Err("Requested observation index is outside the completed sweep".into());
        }
        let count = result.manifest["runs"]
            .as_array()
            .filter(|runs| runs.len() >= 2)
            .ok_or("Missing completed runs")?
            .len();
        let branches=self.requested_branches.unwrap_or([0,count-1]);
        if let Some(pair)=self.requested_branches {
            let d=result.single.as_ref().ok_or("Branch selection requires a single-setting design")?;
            let (a,b)=d.selected(pair[0],pair[1])?;result.a=a.clone();result.b=b.clone();
        }
        // Keep all four actual recordings. First/last copies preserve the existing comparison API.
        let curve=result.paired.as_ref().map(|d|d.curve(self.metric)).transpose()?;
        let path = format!("{}/run-{:03}", result.path,branches[0]);
        let comparison_path = format!("{}/run-{:03}", result.path, branches[1]);
        let loaded_band = cosmogon_sim::research::RunBand::read_optional(std::path::Path::new(&path))?;
        let comparison_band = cosmogon_sim::research::RunBand::read_optional(std::path::Path::new(&comparison_path))?;
        self.loaded_path=path.clone();
        self.path = path;
        self.comparison_path = comparison_path;
        self.loaded_band = loaded_band;
        self.comparison_band = comparison_band;
        self.experiment_mode = self.keep_experiment_at_launch;
        self.keep_experiment_at_launch = false;
        if let Ok(plan) = serde_json::from_value::<Plan>(result.manifest["plan"].clone()) {
            self.sweep.parameter = plan.parameter;
            self.sweep.values = plan.values.iter().map(|v|v.to_string()).collect::<Vec<_>>().join(", ");
            self.sweep.samples = plan.samples;
            self.sweep.interval_hours = plan.interval_s / 3600.0;
            self.sweep.paired = plan.secondary.is_some();
            self.sweep.band = plan.authored_relative_band.map(|b| b.to_string()).unwrap_or_default();
            if let Some(b) = plan.secondary {
                self.sweep.secondary_parameter = b.parameter;
                self.sweep.secondary_values = b.values.iter().map(|v|v.to_string()).collect::<Vec<_>>().join(", ");
            }
            // Reopening a plan does not nominate its existing output for another write.
            self.sweep.output.clear();
        }
        self.balance.reset();
        if result.origin_selection.is_none(){self.sweep_request=None;}
        self.use_saved_origin=self.requested_origin.take().or(result.origin_selection).unwrap_or(true);
        self.origin=result.origin;self.origin_validation=None;
        self.recording = Some(result.a);
        self.comparison = Some(result.b);
        self.selected = index;
        self.factorial_mode = result.paired.is_some();
        self.paired_curve=curve.map(|points|(self.metric,points));
        self.replay.reset();
        self.paired_source=if result.paired.is_some() {result.path.clone()}else{String::new()};
        self.single_source=if result.single.is_some(){result.path.clone()}else{String::new()};
        self.branches=branches;
        self.requested_branches=None;
        self.single=result.single;
        self.paired=result.paired;
        self.sweep_manifest = Some(result.manifest);
        self.status = Some(format!(
            "Completed sweep in {}. Runs {} and {} opened; every branch is retained.",
            result.path,branches[0],branches[1]
        ));
        Ok(())
    }
    fn commit(&mut self, r: Recording, path: String, sample: Option<usize>) -> Result<(), String> {
        let selected = sample.unwrap_or(0);
        if selected >= r.rows.len() {
            return Err("Requested observation index is outside the recording".into());
        }
        self.balance.reset();
        self.sweep_manifest = None;
        self.paired=None;self.single=None;self.origin=None;self.origin_validation=None;self.use_saved_origin=false;self.sweep_request=None;self.single_source.clear();self.paired_curve=None;self.replay.reset();
        self.factorial_mode = false;
        self.recording = Some(r);
        self.loaded_path=path.clone();
        self.loaded_band = cosmogon_sim::research::RunBand::read_optional(std::path::Path::new(&path))?;
        self.path = path;
        self.selected = selected;
        self.status = Some("Recorded observations reopened. The live world is unchanged.".into());
        Ok(())
    }
    pub fn poll(&mut self) {
        self.balance.poll();self.figure.poll();self.pressure.poll();self.why.poll();self.replay.poll();
        if let Some(result)=self.paired_export.as_mut().and_then(|task|block_on(future::poll_once(task))) {
            self.paired_export=None;
            self.status=Some(match result {Ok(())=>"Protected paired trajectory analysis exported; observations are unchanged.".into(),Err(e)=>format!("Trajectory export failed: {e}. Recorded views are preserved.")});
        }
        let result = self
            .task
            .as_mut()
            .and_then(|task| block_on(future::poll_once(task)));
        if let Some(result) = result {
            self.task = None;
            match result
                .and_then(|r| self.commit(r, self.pending_path.clone(), self.requested_sample))
            {
                Ok(()) => {}
                Err(e) => {
                    self.cancel_dependent_launches();
                    self.status = Some(format!(
                        "Could not open recording: {e}. Previous observations are preserved."
                    ))
                }
            }
        }
        if let Some(result) = self
            .comparison_task
            .as_mut()
            .and_then(|task| block_on(future::poll_once(task)))
        {
            self.comparison_task = None;
            match result {
                Ok(r) => match cosmogon_sim::research::RunBand::read_optional(std::path::Path::new(&self.pending_comparison_path)) {
                    Ok(band) => {
                        self.comparison_band = band;
                        self.balance.reset();
                        self.sweep_manifest = None;
                        self.paired=None;self.single=None;self.origin=None;self.origin_validation=None;self.use_saved_origin=false;self.sweep_request=None;self.single_source.clear();self.paired_curve=None;self.replay.reset();
                        self.factorial_mode = false;
                        self.comparison = Some(r);
                        self.comparison_path = self.pending_comparison_path.clone();
                        self.status=Some("Comparison observations reopened. Exact stored-time alignment; no live-world change.".into());
                    }
                    Err(e) => {
                        self.status = Some(format!(
                            "Could not open comparison band metadata: {e}. Previous comparison is preserved."
                        ));
                    }
                },
                Err(e) => {
                    self.status = Some(format!(
                        "Could not open comparison: {e}. Previous comparison is preserved."
                    ))
                }
            }
        }
        if let Some(result) = self
            .sweep_task
            .as_mut()
            .and_then(|task| block_on(future::poll_once(task)))
        {
            self.sweep_task = None;
            if let Err(e) = result.and_then(|r| self.commit_sweep(r)) {
                self.cancel_dependent_launches();
                self.status=Some(format!("Sweep failed: {e}. Previous recorded views are preserved. A folder without completion manifest is incomplete."));
            }
        }
        if (self.balance_mode || self.balance.launch) && self.launch.is_none() && self.rerun_output.is_none() && self.task.is_none() && self.sweep_task.is_none() && self.comparison_task.is_none() {
            if !self.balance.prepared {let b=self.prepare_balance();self.balance.set(b);}
            if self.balance.launch {let source=if self.paired.is_some(){self.paired_source.clone()}else{self.loaded_path.clone()};self.balance.start(&source);}
        }
        if self.figure.launch && self.launch.is_none() && self.rerun_output.is_none() && self.task.is_none() && self.sweep_task.is_none() && self.comparison_task.is_none() {
            let figure=self.prepare_figure();self.figure.start(figure);
        }
        if self.replay.launch && self.rerun_output.is_none() && (self.paired.is_some() || self.single.is_some()) && self.task.is_none() && self.comparison_task.is_none() && self.sweep_task.is_none() {
            self.replay.start(if self.single.is_some(){self.single_source.clone()}else{self.paired_source.clone()});
        }
    }
    fn prepare_balance(&self)->Result<cosmogon_sim::habitat_balance::Bundle,String> {
        let r=self.recording.as_ref().ok_or("Open a recording or paired design first")?;
        let end=self.balance.to.unwrap_or(r.rows.len().saturating_sub(1));
        if let Some(d)=&self.paired {cosmogon_sim::habitat_balance::Bundle::paired(d,self.balance.from,end)}
        else {cosmogon_sim::habitat_balance::Bundle::ordinary(r,self.balance.from,end)}
    }
    fn prepare_figure(&self)->Result<cosmogon_sim::habitat_figure::Figure,String> {
        let mut layout=self.graph.layout.clone();layout.metric=self.metric.id().into();layout.effect=self.effect.id().into();layout.alignment=match self.alignment {Alignment::Elapsed=>"elapsed",Alignment::World=>"world"}.into();
        if self.factorial_mode {cosmogon_sim::habitat_figure::Figure::paired(self.paired.as_ref().ok_or("Open a paired design first")?,&layout)}
        else if let Some(d)=&self.single {d.figure(self.branches[0],self.branches[1],&layout)}
        else {cosmogon_sim::habitat_figure::Figure::ordinary(self.recording.as_ref().ok_or("Open a recording first")?,self.comparison.as_ref(),&layout)}
    }
    fn select_branches(&mut self, branches: [usize;2]) -> Result<(),String> {
        let d=self.single.as_ref().ok_or("Open a single-setting design first")?;
        let (a,b)=d.selected(branches[0],branches[1])?;
        let (a,b)=(a.clone(),b.clone());
        let loaded_path=format!("{}/run-{:03}",self.single_source,branches[0]);
        let comparison_path=format!("{}/run-{:03}",self.single_source,branches[1]);
        let loaded_band=cosmogon_sim::research::RunBand::read_optional(std::path::Path::new(&loaded_path))?;
        let comparison_band=cosmogon_sim::research::RunBand::read_optional(std::path::Path::new(&comparison_path))?;
        self.loaded_path=loaded_path.clone();
        self.path=loaded_path;
        self.comparison_path=comparison_path;
        self.loaded_band=loaded_band;
        self.comparison_band=comparison_band;
        self.recording=Some(a);self.comparison=Some(b);self.branches=branches;
        self.balance.reset();
        Ok(())
    }
    pub fn metadata(&self) -> Option<serde_json::Value> {
        let r = self.recording.as_ref()?;
        let row = &r.rows[self.selected.min(r.rows.len() - 1)];
        let mut metadata = serde_json::json!({"summary":r.summary,"metric":self.metric.id(),"unit":self.metric.unit(),"selected_observation":self.selected,"selected_row":row,"why":self.why_mode.then(||self.why.metadata(&row.habitat,row.simulation_time_s,"recorded_completed_interval")),"selected_value":self.metric.value(row),"comparison":self.comparison.as_ref().map(|b| {let pairs=habitat_comparison::pairs(r,b,self.metric,self.alignment);let pair=&pairs[self.selected.min(pairs.len()-1)];serde_json::json!({"summary":b.summary,"alignment":self.alignment,"selected_pair":pair,"selected_row":pair.b_index.map(|j|&b.rows[j]),"report":habitat_comparison::report_with_bands(r,b,self.metric,self.alignment,self.loaded_band.as_ref(),self.comparison_band.as_ref())})}),"sweep":self.sweep_manifest,"staged_geometry_thermal_plan":(self.experiment_mode && (self.sweep.parameter.geometry_or_thermal() || self.sweep.paired && self.sweep.secondary_parameter.geometry_or_thermal())).then(||serde_json::json!({"parameter":self.sweep.parameter.id(),"values_text":self.sweep.values,"paired":self.sweep.paired,"secondary_parameter":self.sweep.secondary_parameter.id(),"secondary_values_text":self.sweep.secondary_values,"samples":self.sweep.samples,"interval_hours":self.sweep.interval_hours,"new_output":self.sweep.output,"scope":"Editable staged inputs for a new experiment; completed recording inputs remain separate"})),"pressure":self.pressure_mode.then(||self.pressure.metadata(r,self.selected,&self.loaded_path)),"balance":self.balance_mode.then(||self.balance.metadata()),"paired_results_visible":self.factorial_mode && !self.pressure_mode && !self.figure_mode && !self.balance_mode,"graph_view":self.graph_metadata(),"graph_layout_visible":self.graph_mode,"figure":self.figure_mode.then(||serde_json::json!({"graph":self.graph_metadata(),"export":self.figure.metadata(),"paired_contrast":self.factorial_mode})),"replay":self.replay.metadata(),"paired_source":self.paired.as_ref().map(|_|&self.paired_source),"paired_selected_analysis":self.paired.as_ref().and_then(|d|d.report_at(self.selected).ok()),"paired_curve":self.paired_curve.as_ref().filter(|(m,_)|*m==self.metric).map(|(_,points)|serde_json::json!({"metric":self.metric.id(),"effect":self.effect.id(),"point_count":points.len(),"available_count":points.iter().filter(|p|self.effect.value(&p.contrasts).is_some()).count(),"selected_point":points.get(self.selected),"pane":self.paired_pane})),"scope":"Independent recorded view at capture request, possibly hidden; no live-world or camera change"});
        if self.metric.is_heat() {
            let recordings = if self.factorial_mode { self.paired.as_ref().map(|d| d.recordings.iter().collect::<Vec<_>>()).unwrap_or_else(|| vec![r]) } else { std::iter::once(r).chain(self.comparison.as_ref()).collect() };
            metadata["heat_trajectory_context"] = cosmogon_sim::habitat_heat_trajectory::context(&recordings);
        }
        if self.experiment_mode && (self.sweep.parameter.crop() || self.sweep.paired && self.sweep.secondary_parameter.crop()) {
            metadata["staged_crop_plan"] = serde_json::json!({"parameter":self.sweep.parameter.id(),"values_text":self.sweep.values,"paired":self.sweep.paired,"secondary_parameter":self.sweep.secondary_parameter.id(),"secondary_values_text":self.sweep.secondary_values,"samples":self.sweep.samples,"interval_hours":self.sweep.interval_hours,"new_output":self.sweep.output,"scope":"Editable inputs for a new installed crop assumption study; completed recording inputs remain separate"});
        }
        if self.metric.has_crop_context() {
            let recordings = if self.factorial_mode { self.paired.as_ref().map(|d| d.recordings.iter().collect::<Vec<_>>()).unwrap_or_else(|| vec![r]) } else { std::iter::once(r).chain(self.comparison.as_ref()).collect() };
            metadata["crop_trajectory_context"] = cosmogon_sim::habitat_crop_trajectory::context(&recordings);
        }
        if let Some(d)=&self.single {metadata["single_design"]=serde_json::json!({"source":self.single_source,"manifest":d.manifest,"selected_branches":self.branches,"observation":d.report_at(self.selected).ok()});}
        if let Some(origin)=&self.origin {
            metadata["study_origin"]=serde_json::json!({"retained_original":origin.metadata(),"selected_kind":if self.use_saved_origin{"original_saved_common_world"}else{"current_live_world"},"staged_plan_only":self.experiment_mode,"request":self.sweep_request});
        }
        Some(metadata)
    }
}
fn value_text(value: Option<f64>) -> String {
    value
        .map(|v| {
            if v != 0. && (v.abs() < 1e-5 || v.abs() > 1e8) {
                format!("{v:.12e}")
            } else {
                format!("{v:.9}")
            }
        })
        .unwrap_or_else(|| "unavailable".into())
}
fn crop_scope(ui: &mut egui::Ui, rows: &[&cosmogon_sim::habitat::Sample]) {
    if let Some(row) = rows.first() {
        let context = format!("{}\n\nRetained observations:\n{}",cosmogon_sim::habitat_crop_trajectory::SCOPE,rows.iter().enumerate().map(|(i,r)|format!("{i}: {}",cosmogon_sim::habitat_crop_trajectory::label(r))).collect::<Vec<_>>().join("\n"));
        let prefix=if rows.len()==4 {"A0/B0: "}else if rows.len()==2 {"A: "}else{""};
        ui.label(egui::RichText::new(format!("{prefix}{}",cosmogon_sim::habitat_crop_trajectory::label(row))).small().color(MUTED)).on_hover_text(context);
        ui.label(egui::RichText::new("Reference progress is not biological age; dry harvest supplies no crew diet.").small().color(MUTED));
    }
}
fn heat_scope(ui: &mut egui::Ui, rows: &[&cosmogon_sim::habitat::Sample], unit: &str) {
    if let Some(row) = rows.first() {
        let context = format!("{}\n\nIndividual retained coverage:\n{}", cosmogon_sim::habitat_heat_trajectory::SCOPE, rows.iter().enumerate().map(|(i,row)| format!("{i}: {}",cosmogon_sim::habitat_heat_trajectory::label(row))).collect::<Vec<_>>().join("\n"));
        ui.label(egui::RichText::new(format!("[{unit}] {}", cosmogon_sim::habitat_heat_trajectory::label(row))).small().color(MUTED)).on_hover_text(context);
    }
}
fn ordinary_series(r:&Recording,b:Option<&Recording>,alignment:Alignment,metric:Metric)->Vec<Vec<super::habitat_graph::Datum>> {
    std::iter::once(r).chain(b).map(|s|s.rows.iter().enumerate().map(|(index,row)|super::habitat_graph::Datum {index,coordinate_s:alignment.coordinate(row.simulation_time_s,s.rows[0].simulation_time_s),value:metric.value(row)}).collect()).collect()
}
fn paired_series(points:&[Point],effect:Effect,alignment:Alignment)->Vec<Vec<super::habitat_graph::Datum>> {
    let first=points.first().map(|p|p.simulation_time_s).unwrap_or(0.);
    vec![points.iter().map(|p|super::habitat_graph::Datum {index:p.observation_index,coordinate_s:alignment.coordinate(p.simulation_time_s,first),value:effect.value(&p.contrasts)}).collect()]
}
pub fn panel(
    ui: &mut egui::Ui,
    study: &mut Study,
    world: &Universe,
    habitat_id: &mut Option<u64>,
    reference: bool,
) {
    ui.horizontal(|ui| {
        ui.selectable_value(
            &mut study.experiment_mode,
            false,
            "Recordings & comparisons",
        );
        ui.selectable_value(&mut study.experiment_mode, true, "Run experiment");
    });
    if !study.experiment_mode {
        let current=if study.balance_mode {6}else if study.figure_mode {5}else if study.pressure_mode {4}else if study.graph_mode {3}else if study.factorial_mode {2}else if study.why_mode {1}else{0};
        let mut chosen=current;
        let labels=["Curves & comparison","WHY selected interval","Paired results","Graph layout","Pressure components","Research figure","Cabin balances"];
        egui::ComboBox::from_id_salt("recorded-habitat-view").selected_text(labels[current]).show_ui(ui,|ui|{
            for (index,label) in labels.iter().enumerate() {
                if index==2 && !study.sweep_manifest.as_ref().is_some_and(|m|m["factorial_analysis"].is_object()) {continue;}
                if index>=3 && study.recording.is_none() {continue;}
                ui.selectable_value(&mut chosen,index,*label);
            }
        });
        if chosen!=current {
            study.balance_mode=chosen==6;
            study.figure_mode=chosen==5;
            study.pressure_mode=chosen==4;
            study.graph_mode=chosen==3;
            study.why_mode=chosen==1;
            if chosen<=2 {study.factorial_mode=chosen==2;}
        }
    }
    if study.single.is_some() && !study.experiment_mode {
        let mut branches=study.branches;
        let d=study.single.as_ref().unwrap();
        ui.add(egui::Label::new(egui::RichText::new(format!("Saved single-setting design: {}",study.single_source)).small().color(MUTED)).wrap());
        let plan=d.plan().expect("checked design plan");
        ui.add(egui::Label::new(format!("Varied setting: {} ({})",plan.parameter.label(),plan.parameter.unit())).wrap());
        ui.horizontal_wrapped(|ui| {
            for (side,branch) in branches.iter_mut().enumerate() {
                let label=|i:usize|format!("{}: run-{i:03} · {} {}",if side==0{"A"}else{"B"},plan.values[i],plan.parameter.unit());
                egui::ComboBox::from_id_salt(("saved-single-branch",side)).selected_text(label(*branch)).show_ui(ui,|ui| {
                    for i in 0..d.recordings.len(){ui.selectable_value(branch,i,label(i));}
                });
            }
        });
        if branches!=study.branches {if let Err(e)=study.select_branches(branches){study.status=Some(e);}}
        egui::CollapsingHeader::new("Verify complete saved design").show(ui,|ui| {
            study.replay.show(ui,&study.single_source,study.task.is_some() || study.sweep_task.is_some() || study.comparison_task.is_some());
        });
    }
    if study.balance_mode && !study.experiment_mode {
        let source=if study.paired.is_some(){study.paired_source.clone()}else{study.loaded_path.clone()};let count=study.recording.as_ref().map_or(0,|r|r.rows.len());
        ui.scope(|ui|{ui.spacing_mut().item_spacing.y=3.;study.balance.show(ui,&source,count,study.task.is_some() || study.sweep_task.is_some() || study.comparison_task.is_some());});
        return;
    }
    if study.figure_mode && !study.experiment_mode {
        ui.scope(|ui| {
        ui.spacing_mut().item_spacing.y=3.0;
        let source=if study.factorial_mode {&study.paired_source}else{&study.loaded_path};
        ui.add(egui::Label::new(egui::RichText::new(format!("Loaded source: {source}")).small().color(MUTED)).truncate()).on_hover_text(source);
        if study.paired.is_some() {ui.checkbox(&mut study.factorial_mode,"Paired contrast (all four branches)");}
        ui.horizontal_wrapped(|ui| {
            egui::ComboBox::from_id_salt("figure-metric").selected_text(format!("{} ({})",study.metric.label(),study.metric.unit())).show_ui(ui,|ui|{for m in Metric::ALL {ui.selectable_value(&mut study.metric,m,m.label());}});
            if study.factorial_mode {egui::ComboBox::from_id_salt("figure-effect").selected_text(study.effect.label()).show_ui(ui,|ui|{for e in Effect::ALL {ui.selectable_value(&mut study.effect,e,e.label());}});}
        });
        ui.horizontal_wrapped(|ui| {
            ui.selectable_value(&mut study.alignment, Alignment::Elapsed, "Elapsed time");
            ui.selectable_value(&mut study.alignment, Alignment::World, "World time");
            egui::ComboBox::from_id_salt("figure-time-unit").selected_text(study.graph.layout.time_unit.label()).show_ui(ui, |ui| {
                for t in [super::habitat_graph::TimeUnit::Seconds, super::habitat_graph::TimeUnit::Hours, super::habitat_graph::TimeUnit::Days] {ui.selectable_value(&mut study.graph.layout.time_unit,t,t.label());}
            });
            ui.checkbox(&mut study.graph.layout.include_zero,"Include zero");
        });
        if study.graph.layout.scale==super::habitat_graph::Scale::SignedLog {
            ui.horizontal(|ui| {ui.label("Signed threshold, metric units");ui.add(egui::DragValue::new(&mut study.graph.layout.signed_threshold).range(1e-12..=1e12).speed(0.01));});
        }
        study.sync_graph_layout();
        if let Some(recording)=&study.recording {
            let data=if study.factorial_mode {study.paired.as_ref().and_then(|d|d.curve(study.metric).ok()).map(|p|paired_series(&p,study.effect,study.alignment)).unwrap_or_default()}else{ordinary_series(recording,study.comparison.as_ref(),study.alignment,study.metric)};
            if study.metric.is_heat() {
                let rows = if study.factorial_mode { study.paired.as_ref().map(|d| d.recordings.iter().filter_map(|r| r.rows.get(study.selected)).collect::<Vec<_>>()).unwrap_or_default() } else { std::iter::once(recording).chain(study.comparison.as_ref()).filter_map(|r| r.rows.get(study.selected)).collect() };
                heat_scope(ui, &rows, study.metric.unit());
            }
            if study.metric.has_crop_context() {
                let rows: Vec<_> = if study.factorial_mode { study.paired.as_ref().map(|d| d.recordings.iter().filter_map(|r| r.rows.get(study.selected)).collect()).unwrap_or_default() } else { std::iter::once(recording).chain(study.comparison.as_ref()).filter_map(|r| r.rows.get(study.selected)).collect() };
                crop_scope(ui,&rows);
            }
            study.graph.show(ui,&data,&mut study.selected,study.metric.unit(),if study.metric.is_heat() || study.metric.is_crop(){90.}else{100.},true);
            let busy=study.task.is_some() || study.sweep_task.is_some() || study.comparison_task.is_some();
            if study.figure.controls(ui,busy) {let figure=study.prepare_figure();study.figure.start(figure);}
        }else{ui.label("Open an actual recording first.");}
        });
        return;
    }
    if study.pressure_mode && !study.experiment_mode {
        if let Some(r)=&study.recording {ui.scope(|ui|{ui.spacing_mut().item_spacing.y=3.0;study.pressure.show(ui,r,&mut study.selected,&study.loaded_path,study.paired.is_some());});}else{ui.label("Open a recording in Curves & comparison first.");}
        return;
    }
    study.sync_graph_layout();
    if study.graph_mode && !study.experiment_mode {
        ui.horizontal_wrapped(|ui|{
            egui::ComboBox::from_id_salt("layout-metric").selected_text(format!("{} ({})",study.metric.label(),study.metric.unit())).show_ui(ui,|ui|{for metric in Metric::ALL {ui.selectable_value(&mut study.metric,metric,metric.label());}});
            if study.paired.is_some(){egui::ComboBox::from_id_salt("layout-effect").selected_text(study.effect.label()).show_ui(ui,|ui|{for effect in Effect::ALL {ui.selectable_value(&mut study.effect,effect,effect.label());}});}
            ui.selectable_value(&mut study.alignment,Alignment::Elapsed,"Elapsed time");ui.selectable_value(&mut study.alignment,Alignment::World,"World time");
        });
        study.sync_graph_layout();if study.graph.settings(ui){study.apply_graph_layout();}
        if ui.button("Return to graph").clicked(){study.graph_mode=false;study.paired_pane=0;study.why_mode=false;}
        return;
    }
    if study.factorial_mode && !study.experiment_mode {ui.scope(|ui|{ui.spacing_mut().item_spacing.y=3.0;paired_results(ui,study);});return;}

    if study.why_mode && !study.experiment_mode {
        ui.label(egui::RichText::new("Recorded evidence · independent of the live world").color(ACCENT));
        if let Some(r)=&study.recording {
            study.selected=study.selected.min(r.rows.len()-1);
            ui.label(format!("A: {} · {} recorded observations",r.summary["name"].as_str().unwrap_or("Habitat"),r.rows.len()));
            ui.add(egui::Slider::new(&mut study.selected,0..=r.rows.len()-1).text("Observation index"));
            let row=&r.rows[study.selected];
            study.why.show(ui,&row.habitat,row.simulation_time_s,false);
            ui.label(egui::RichText::new("This explains A's exact saved interval. Return to Curves & comparison for supported B−A differences; no unique cause is attributed across changed initial conditions.").small().color(MUTED));
        } else {ui.label("Open a recording in Curves & comparison first.");}
        return;
    }
    if study.experiment_mode {
        ui.scope(|ui|{ui.spacing_mut().item_spacing.y=3.0;experiment(ui, study, world, habitat_id, reference);});
        if let Some(status) = &study.status {
            ui.colored_label(ACCENT, status);
        }
        return;
    }
    if study.sweep_manifest.as_ref().is_some_and(|m|m["factorial_analysis"].is_object()) {ui.colored_label(ACCENT,"Both settings change between these corners; use Paired results to isolate effects.");}
    ui.label(
        egui::RichText::new("Actual exported observations · independent of the live world")
            .color(ACCENT),
    );
    if let Some(r) = &study.recording {
        ui.label(format!(
            "A: {} · world [{},{}] · {} stored observations",
            r.summary["name"].as_str().unwrap_or("Habitat"),
            r.rows[0].habitat.definition.body.system,
            r.rows[0].habitat.definition.body.body,
            r.rows.len()
        ));
        egui::ComboBox::from_id_salt("habitat-recording-metric")
            .selected_text(format!("{} ({})",study.metric.label(),study.metric.unit()))
            .show_ui(ui, |ui| {
                for metric in Metric::ALL {
                    ui.selectable_value(&mut study.metric, metric, metric.label());
                }
            });
        if let Some(b) = &study.comparison {
            ui.label(format!(
                "B: {} · {} observations",
                b.summary["name"].as_str().unwrap_or("Habitat"),
                b.rows.len()
            ));
            ui.horizontal(|ui| {
                ui.colored_label(ACCENT, "● A");
                ui.colored_label(egui::Color32::from_rgb(91, 196, 230), "■ B");
                ui.selectable_value(&mut study.alignment, Alignment::Elapsed, "Elapsed time");
                ui.selectable_value(&mut study.alignment, Alignment::World, "World time");
            });
        }
        let data=ordinary_series(r,study.comparison.as_ref(),study.alignment,study.metric);
        study.graph.layout.metric=study.metric.id().into();
        study.graph.layout.alignment=match study.alignment{Alignment::Elapsed=>"elapsed",Alignment::World=>"world"}.into();
        ui.label(format!("{} ({}) vs {}",study.metric.label(),study.metric.unit(),if study.alignment==Alignment::Elapsed{"elapsed time"}else{"world time since J2000"}));
        if study.metric.is_heat() { heat_scope(ui, &std::iter::once(r).chain(study.comparison.as_ref()).filter_map(|r| r.rows.get(study.selected)).collect::<Vec<_>>(), study.metric.unit()); }
        if study.metric.has_crop_context() {crop_scope(ui,&std::iter::once(r).chain(study.comparison.as_ref()).filter_map(|r|r.rows.get(study.selected)).collect::<Vec<_>>());}
        study.graph.show(ui,&data,&mut study.selected,study.metric.unit(),120.,true);
        ui.add(
            egui::Slider::new(&mut study.selected, 0..=r.rows.len() - 1).text("Observation index"),
        );
        let row = &r.rows[study.selected];
        ui.strong(format!(
            "{}: {} {}",
            study.metric.label(),
            study
                .metric
                .value(row)
                .map(|v| if v != 0.0 && (v.abs() < 1e-5 || v.abs() > 1e8) {
                    format!("{v:.12e}")
                } else {
                    format!("{v:.9}")
                })
                .unwrap_or_else(|| "unavailable".into()),
            study.metric.unit()
        ));
        ui.label(format!(
            "World time {:.3} s · completed habitat time {:.3} s · interval {:.3} s",
            row.simulation_time_s, row.habitat.last_time, row.habitat.budget.seconds
        ));
        if let Some(b) = &study.comparison {
            let pairs = habitat_comparison::pairs(r, b, study.metric, study.alignment);
            let pair = &pairs[study.selected];
            ui.strong(format!(
                "B: {} {} · B−A: {} {}",
                value_text(pair.b_value),
                study.metric.unit(),
                value_text(pair.b_minus_a),
                study.metric.unit()
            ));
            ui.label(format!(
                "Comparison support: {} · B world time {} s",
                match pair.support {
                    "matched" => "Matched stored observation",
                    "no_exact_timestamp" => "No observation at this exact time",
                    "different_heat_origin" => "Different heat origins; subtraction unavailable",
                    "different_heat_completed_end" => "Different heat completion times; subtraction unavailable",
                    "incomplete_heat_coverage" => "Excluded heat intervals; subtraction unavailable",
                    "different_heat_execution" => "Different executed heat duration; subtraction unavailable",
                    "different_interval_support" =>
                        "Different completed intervals; subtraction unavailable",
                    _ => "One recorded value is unavailable",
                },
                pair.b_world_time_s
                    .map(|t| format!("{t:.3}"))
                    .unwrap_or_else(|| "no matching observation".into())
            ));
            let ah = &r.rows[0].habitat;
            let bh = &b.rows[0].habitat;
            if ah.definition != bh.definition || ah.config != bh.config || ah.state != bh.state {
                ui.label(
                    egui::RichText::new(
                        "Initial conditions differ; inspect settings before attributing a cause.",
                    )
                    .small()
                    .color(ACCENT),
                );
            }
            egui::CollapsingHeader::new("Compare original conditions and export differences").show(
                ui,
                |ui| {
                    let report = habitat_comparison::report_with_bands(r, b, study.metric, study.alignment, study.loaded_band.as_ref(), study.comparison_band.as_ref());
                    ui.label(format!(
                        "Initial conditions differ: {} · source versions differ: {}",
                        report["initial_conditions_differ"], report["source_versions_differ"]
                    ));
                    if let Some(unavailable) = report["uncertainty"]["unavailable"].as_str() {
                        ui.colored_label(ACCENT, format!("Band envelope unavailable: {unavailable}"));
                    } else if report["uncertainty"].is_object() {
                        ui.label("Authored input-band interval arithmetic is included in the comparison JSON below.");
                    }
                    ui.label(report["limits"].as_str().unwrap_or_default());
                    ui.label(format!("A: {} · B: {}", study.path, study.comparison_path));
                    if ui.button("Copy complete comparison JSON").clicked() {
                        ui.ctx()
                            .copy_text(serde_json::to_string_pretty(&report).unwrap_or_default());
                    }
                    ui.label("New comparison output folder");
                    ui.text_edit_singleline(&mut study.comparison_output);
                    if ui.button("Export actual comparison JSON and CSV").clicked() {
                        study.status = Some(
                            match habitat_comparison::export_with_bands(
                                r,
                                b,
                                study.metric,
                                study.alignment,
                                study.loaded_band.as_ref(),
                                study.comparison_band.as_ref(),
                                std::path::Path::new(&study.comparison_output),
                            ) {
                                Ok(()) => {
                                    "Comparison exported; prior folders are protected.".into()
                                }
                                Err(e) => e,
                            },
                        );
                    }
                    egui::ScrollArea::vertical()
                        .max_height(160.)
                        .show(ui, |ui| {
                            ui.code(serde_json::to_string_pretty(&report).unwrap_or_default());
                        });
                },
            );
        }
        ui.label(egui::RichText::new("Points are recorded samples; connecting lines are visual guides. Missing active intervals are gaps. No interpolation, prediction or world rewind.").small().color(MUTED));
        egui::CollapsingHeader::new("Selected conditions, sources and constraints").show(ui,|ui| {
            ui.label(format!("Original site {:.5}° N, {:.5}° E · crew demand {} · current cabin {:.3} K",row.habitat.definition.latitude_deg,row.habitat.definition.longitude_east_deg,row.habitat.config.crew,row.habitat.state.temperature_k));
            ui.label(format!("Current equipment: electrolysis {} · CO₂ reduction {} · completed interval environment {}",row.habitat.config.electrolysis.is_some(),row.habitat.config.sabatier.is_some(),row.habitat.budget.environment_available));
            if let Some(b)=&row.habitat.budget.electrolysis {ui.label(format!("Electrolysis constraints: {}",b.constraints.iter().map(|c|c.label()).collect::<Vec<_>>().join("; ")));}
            if let Some(b)=&row.habitat.budget.sabatier {ui.label(format!("Reduction constraints: {}",b.constraints.iter().map(|c|c.label()).collect::<Vec<_>>().join("; ")));}
            ui.hyperlink_to("NASA nominal crew model",cosmogon_sim::habitat::SOURCE);
            ui.label(r.summary["validation_scope"].as_str().unwrap_or("Structural checks only"));
            ui.label(format!("Initial identity {} · final identity {}",r.summary["initial_state_fingerprint"],r.summary["final_state_fingerprint"]));
            ui.label("Recorded source/configuration and model limits belong to the original bundle. They are not inferred from the live world, and bookkeeping checks do not establish field accuracy.");
        });
        egui::CollapsingHeader::new("All selected values, configuration and source metadata").show(
            ui,
            |ui| {
                let reports=if let Some(b)=&study.comparison {let pairs=habitat_comparison::pairs(r,b,study.metric,study.alignment);serde_json::json!({"a":row.habitat.report(),"b":pairs[study.selected].b_index.map(|j|b.rows[j].habitat.report())})} else {row.habitat.report()};
                let data = serde_json::to_string_pretty(&reports).unwrap_or_default();
                if ui.button("Copy selected reports JSON").clicked() {
                    ui.ctx().copy_text(data.clone());
                }
                egui::ScrollArea::vertical()
                    .max_height(200.0)
                    .show(ui, |ui| {
                        ui.code(data);
                    });
            },
        );
    } else {
        ui.label("Open a protected habitat-study folder from CLI habitat or the native snapshot export. A snapshot contains one observation; a trajectory contains multiple requested world times.");
    }
    ui.separator();
    ui.label("Study folder");
    ui.text_edit_singleline(&mut study.path);
    if ui
        .add_enabled(
            !study.pending(),
            egui::Button::new("Open recorded observations"),
        )
        .clicked()
    {
        study.start(study.path.clone(), None);
    }
    ui.label("Comparison study folder (B)");
    ui.text_edit_singleline(&mut study.comparison_path);
    ui.horizontal(|ui| {
        if ui
            .add_enabled(
                !study.pending() && study.recording.is_some(),
                egui::Button::new("Open comparison"),
            )
            .clicked()
        {
            study.start_comparison(study.comparison_path.clone());
        }
        if ui
            .add_enabled(
                !study.pending() && study.comparison.is_some(),
                egui::Button::new("Remove comparison"),
            )
            .clicked()
        {
            study.comparison = None;
            study.comparison_band = None;
            study.sweep_manifest = None;
            study.paired=None;study.paired_curve=None;study.replay.reset();study.factorial_mode=false;
        }
    });
    let band_block = match (&study.loaded_band, &study.comparison_band) {
        (Some(a_band), Some(b_band)) => {
            let same_horizon = study.recording.as_ref().zip(study.comparison.as_ref()).is_some_and(|(a, b)| {
                a.rows.last().is_some_and(|r| r.habitat.budget.seconds > 0.0)
                    && a.rows.last().map(|r| r.habitat.budget.seconds)
                        == b.rows.last().map(|r| r.habitat.budget.seconds)
            });
            Some(cosmogon_sim::research::RunBand::comparison_block(a_band, b_band, same_horizon))
        }
        _ => None,
    };
    if let Some(block) = &band_block {
        ui.separator();
        ui.label("Authored input-band envelope (per-run final observations)");
        if let Some(metric_block) = block["metrics"].get(study.metric.id()) {
            if metric_block["unavailable"].is_string() {
                ui.colored_label(ACCENT, metric_block["unavailable"].as_str().expect("string"));
            } else {
                ui.label(format!(
                    "{} · B−A central {:.6e} · interval [{:.6e}, {:.6e}]",
                    study.metric.unit(),
                    metric_block["central_b_minus_a"].as_f64().unwrap_or(f64::NAN),
                    metric_block["at_low"].as_f64().unwrap_or(f64::NAN),
                    metric_block["at_high"].as_f64().unwrap_or(f64::NAN),
                ));
            }
        } else {
            ui.colored_label(ACCENT, "No band triple for the selected metric.");
        }
        ui.label(egui::RichText::new(block["assumption"].as_str().unwrap_or_default()).small().color(MUTED));
    } else if study.loaded_band.is_some() || study.comparison_band.is_some() {
        ui.separator();
        ui.colored_label(ACCENT, "Only one side carries authored band metadata; interval arithmetic needs both.");
    }
    if let Some(status) = &study.status {
        ui.colored_label(ACCENT, status);
    }
}
fn paired_results(ui:&mut egui::Ui,study:&mut Study) {
    if let Some(status)=&study.status {if study.task.is_some() || study.sweep_task.is_some() || study.comparison_task.is_some() || study.paired_export.is_some() || status.contains("failed:") || status.contains("Could not") {ui.colored_label(ACCENT,status);}}
    let Some(design)=&study.paired else {ui.label("Open a completed paired experiment first.");return;};
    let plan=match design.plan() {Ok(p)=>p,Err(e)=>{ui.colored_label(ACCENT,e);return;}};
    study.selected=study.selected.min(plan.samples-1);
    ui.strong("Paired settings · four actual recorded responses");
    ui.horizontal(|ui| {ui.selectable_value(&mut study.paired_pane,0,"Time curves");ui.selectable_value(&mut study.paired_pane,1,"Four-response table");ui.selectable_value(&mut study.paired_pane,2,"Files & method");ui.selectable_value(&mut study.paired_pane,3,"Replay");});
    ui.add(egui::Label::new(format!("A: {} · {:?} {}",plan.parameter.label(),plan.values,plan.parameter.unit())).wrap());
    let second=plan.secondary.as_ref().expect("verified paired design");
    ui.add(egui::Label::new(format!("B: {} · {:?} {}",second.parameter.label(),second.values,second.parameter.unit())).wrap());
    if study.paired_pane==3 {
        let pending=study.task.is_some() || study.sweep_task.is_some() || study.comparison_task.is_some() || study.paired_export.is_some();
        study.replay.show(ui,&study.paired_source,pending);return;
    }
    if study.paired_pane==2 {
        ui.label(format!("All stored times and {} metrics: four responses, eight contrasts, interval support, source/common-state identity and formulas.",plan.metrics(&design.recordings[0].rows[0].habitat).len()));
        if let Some(note)=plan.configuration_assumptions() {assumptions(ui,&note);}
        ui.label("New trajectory analysis folder");ui.text_edit_singleline(&mut study.paired_output);
        if ui.add_enabled(!study.pending() && !study.paired_output.trim().is_empty(),egui::Button::new("Export protected JSON + CSV")).clicked() {
            let source=std::path::PathBuf::from(&study.paired_source);let output=std::path::PathBuf::from(study.paired_output.trim());
            study.paired_export=Some(AsyncComputeTaskPool::get().spawn(async move {cosmogon_sim::habitat_factorial::read(&source)?.export_trajectories(&output)}));
            study.status=Some("Checking source recordings and exporting actual trajectory contrasts…".into());
        }
        if ui.button("Copy selected observation analysis JSON").clicked() {if let Ok(report)=design.report_at(study.selected) {ui.ctx().copy_text(serde_json::to_string_pretty(&report).unwrap_or_default());}}
        if ui.button("Copy original final analysis JSON").clicked() {ui.ctx().copy_text(serde_json::to_string_pretty(&design.analysis).unwrap_or_default());}
        ui.hyperlink_to("NIST factorial design method",cosmogon_sim::habitat_factorial::METHOD_SOURCE);
        ui.label("Conditional modeled changes at the supplied levels. No interpolation, significance, predictive confidence, global optimum or hardware calibration.");
        ui.label("A0/A1 and B0/B1 mean first/second levels, including descending values. Standard AB effect is half the difference of conditional A changes.");
        ui.label(format!("Original four recordings and final factorial.json: {}",study.paired_source));
        if let Some(status)=&study.status {ui.colored_label(ACCENT,status);}return;
    }
    ui.horizontal_wrapped(|ui| {
        egui::ComboBox::from_id_salt("paired-response-metric").selected_text(format!("{} ({})",study.metric.label(),study.metric.unit())).show_ui(ui,|ui| {for metric in Metric::ALL {ui.selectable_value(&mut study.metric,metric,metric.label());}});
        if study.paired_pane==0 {egui::ComboBox::from_id_salt("paired-effect").selected_text(study.effect.label()).show_ui(ui,|ui|{for e in Effect::ALL {ui.selectable_value(&mut study.effect,e,e.label());}});}
    });
    if study.paired_curve.as_ref().is_none_or(|(m,_)|*m!=study.metric) {
        match design.curve(study.metric) {Ok(points)=>study.paired_curve=Some((study.metric,points)),Err(e)=>{ui.colored_label(ACCENT,e);return;}}
    }
    let points=&study.paired_curve.as_ref().expect("prepared curve").1;
    if study.metric.is_heat() { heat_scope(ui, &design.recordings.iter().filter_map(|r| r.rows.get(study.selected)).collect::<Vec<_>>(), study.metric.unit()); }
    if study.metric.has_crop_context() {crop_scope(ui,&design.recordings.iter().filter_map(|r|r.rows.get(study.selected)).collect::<Vec<_>>());}
    if study.paired_pane==0 {
        study.graph.layout.metric=study.metric.id().into();study.graph.layout.effect=study.effect.id().into();
        let data=paired_series(points,study.effect,study.alignment);
        study.graph.show(ui,&data,&mut study.selected,study.metric.unit(),if study.graph.layout.scale==super::habitat_graph::Scale::Linear {135.}else{115.},false);
    }
    ui.add(egui::Slider::new(&mut study.selected,0..=points.len()-1).text("Observation index"));
    let point=&points[study.selected];let c=&point.contrasts;
    ui.label(format!("World time {:.3} s · {}",point.simulation_time_s,study.metric.unit()));
    if point.completed_interval_configurations_match_current.iter().any(|v|!*v) {ui.colored_label(ACCENT,"Flows retain pre-edit settings; changed equipment needs a later interval.");}
    if study.paired_pane==1 {egui::Grid::new("paired-responses").striped(true).show(ui,|ui| {
        ui.label("");ui.strong("A0 (first level)");ui.strong("A1 (second level)");ui.end_row();
        for (label,left,right) in [("B0 (first)",0,1),("B1 (second)",2,3)] {ui.strong(label);ui.label(value_text(c.responses[left]));ui.label(value_text(c.responses[right]));ui.end_row();}
    });}
    if study.paired_pane==0 {ui.strong(format!("{}: {} {}",study.effect.label(),value_text(study.effect.value(c)),study.metric.unit()));}
    else {egui::Grid::new("paired-contrasts").striped(true).show(ui,|ui| {
        for (label,value) in [("A change at B0",c.a_at_b0),("A change at B1",c.a_at_b1),("B change at A0",c.b_at_a0),("B change at A1",c.b_at_a1),("Average A change",c.average_a),("Average B change",c.average_b),("Difference of conditional A changes",c.difference_in_differences),("Standard AB effect (half that difference)",c.standard_ab_effect)] {ui.label(label);ui.strong(value_text(value));ui.end_row();}
    });}
    if point.branch_support.iter().any(|s|*s!="matched") {ui.colored_label(ACCENT,if study.metric.is_heat() {"Contrast unavailable: ledger origin, execution coverage or completed-time mismatch; branch values remain retained."} else if study.metric.is_crop() {"Contrast unavailable: crop origin, completed-time or availability mismatch; branch values remain retained."} else {"Contrast unavailable: missing value or mismatched interval support."});}
    ui.label(egui::RichText::new("Stored model results · gaps remain unavailable · sources/export in Files & method").small().color(MUTED));
}

fn example_levels(ui:&mut egui::Ui,parameter:Parameter,habitat:&cosmogon_sim::habitat::Habitat,values:&mut String) {
    if let Some(levels)=parameter.illustrative_levels_for(habitat) {
        ui.horizontal_wrapped(|ui| {
            if ui.small_button("Use illustrative levels").clicked() {*values=format!("{}, {}",levels[0],levels[1]);}
            ui.label(egui::RichText::new(format!("{}, {} {} · editable assumptions",levels[0],levels[1],parameter.unit())).small().color(MUTED));
        });
    }
}
fn assumptions(ui:&mut egui::Ui,note:&serde_json::Value) {
    ui.colored_label(ACCENT,match note["context_kind"].as_str() {
        Some("crop") => "Fixed installed crop and stocks · assumed chamber, power and capacity",
        Some("crop_and_geometry_thermal") => "Fixed installed crop and stocks · assumed crop and geometry/thermal properties",
        _ => "Fixed initial stocks and temperature · assumed geometry/thermal properties",
    });
    egui::CollapsingHeader::new("What these changes mean").show(ui,|ui| {
        if let Some(settings)=note["settings"].as_array() {for setting in settings {ui.label(setting["assumption"].as_str().unwrap_or(""));}}
        ui.label(note["initial_controls"].as_str().unwrap_or(""));
        ui.label(note["capacity_scope"].as_str().unwrap_or(""));
        if let Some(scope) = note["initial_crop_controls"].as_str() {ui.label(scope);}
        if let Some(scope) = note["crop_scope"].as_str() {ui.label(scope);}
        if let Some(sources)=note["sources"].as_array() {for source in sources {ui.hyperlink_to(source["scope"].as_str().unwrap_or("Source"),source["url"].as_str().unwrap_or(""));}}
        ui.label(note["limits"].as_str().unwrap_or(""));
    });
}
fn experiment(
    ui: &mut egui::Ui,
    study: &mut Study,
    world: &Universe,
    habitat_id: &mut Option<u64>,
    reference: bool,
) {
    ui.strong("Controlled habitat experiment");
    ui.label("Independent branches · fixed initial stocks · assumed properties");
    if let Some(origin)=&study.origin {
        ui.horizontal_wrapped(|ui| {
            ui.selectable_value(&mut study.use_saved_origin,true,"Original saved starting world");
            ui.selectable_value(&mut study.use_saved_origin,false,"Current live world");
        });
        if study.use_saved_origin {
            ui.add(egui::Label::new(format!("Saved source: {} · {} · world time {:.6} s",origin.directory.display(),origin.habitat.definition.name,origin.cached_world().time)).wrap());
            ui.label(egui::RichText::new("Exact original finite stores and clock; the live world stays independent.").small().color(MUTED));
            ui.label(format!("Pinned state {} · ordinary new edits apply before sampling",origin.fingerprint));
        }
    }
    if !study.use_saved_origin || study.origin.is_none() {
    ui.label(format!("Live world time {:.6} s · current finite stores captured when Run is pressed",world.time));
    egui::ComboBox::from_id_salt("sweep-habitat")
        .selected_text(
            habitat_id
                .and_then(|id| world.habitats.iter().find(|h| h.id == id))
                .map(|h| h.definition.name.as_str())
                .unwrap_or("Select a live habitat"),
        )
        .show_ui(ui, |ui| {
            for h in &world.habitats {
                ui.selectable_value(
                    habitat_id,
                    Some(h.id),
                    format!("{} · #{}", h.definition.name, h.id),
                );
            }
        });
    }
    egui::ComboBox::from_id_salt("habitat-sweep-parameter")
        .selected_text(study.sweep.parameter.label())
        .show_ui(ui, |ui| {
            for p in Parameter::ALL {
                ui.selectable_value(&mut study.sweep.parameter, p, p.label());
            }
        });
    ui.label(format!(
        "Distinct values, separated by commas ({})",
        study.sweep.parameter.unit()
    ));
    ui.text_edit_singleline(&mut study.sweep.values);
    ui.horizontal(|ui| {
        ui.label("Authored input band (± fraction, 0 = off)");
        ui.text_edit_singleline(&mut study.sweep.band);
    });
    if let Some(h)=if study.use_saved_origin && study.origin.is_some() {study.origin.as_ref().map(|o|&o.habitat)}else{habitat_id.and_then(|id|world.habitats.iter().find(|h|h.id==id))} {
        example_levels(ui,study.sweep.parameter,h,&mut study.sweep.values);
    }
    ui.checkbox(&mut study.sweep.paired,"Pair with a second setting (four combinations)");
    if study.sweep.paired {
        egui::ComboBox::from_id_salt("habitat-second-setting").selected_text(study.sweep.secondary_parameter.label()).show_ui(ui,|ui| {
            for p in Parameter::ALL {ui.selectable_value(&mut study.sweep.secondary_parameter,p,p.label());}
        });
        ui.label(format!("Exactly two levels for each setting · second levels ({})",study.sweep.secondary_parameter.unit()));
        ui.text_edit_singleline(&mut study.sweep.secondary_values);
        if let Some(h)=if study.use_saved_origin && study.origin.is_some() {study.origin.as_ref().map(|o|&o.habitat)}else{habitat_id.and_then(|id|world.habitats.iter().find(|h|h.id==id))} {
            example_levels(ui,study.sweep.secondary_parameter,h,&mut study.sweep.secondary_values);
        }
    }

    ui.horizontal_wrapped(|ui| {
        ui.label("Observations/run");
        ui.add(egui::DragValue::new(&mut study.sweep.samples).range(1..=1024));
        ui.label("Interval (terrestrial hours)");
        ui.add(
            egui::DragValue::new(&mut study.sweep.interval_hours)
                .range(1. / 3600. ..=720.)
                .speed(0.01)
                .max_decimals(9),
        );
    });
    ui.label("New output folder (all runs and common state)");
    ui.text_edit_singleline(&mut study.sweep.output);
    let values = study
        .sweep
        .values
        .split(',')
        .map(|s| {
            s.trim()
                .parse::<f64>()
                .map_err(|_| "Enter finite comma-separated values".to_string())
        })
        .collect::<Result<Vec<_>, _>>();
    let plan = values.and_then(|values| {
        let band = if study.sweep.band.trim().is_empty() || study.sweep.band.trim() == "0" {
            None
        } else {
            Some(
                study
                    .sweep
                    .band
                    .trim()
                    .parse::<f64>()
                    .map_err(|_| "Authored band must be a finite number".to_string())?,
            )
        };
        let p = Plan {
            format: "cosmogon-habitat-sweep-plan".into(),
            schema_version: if study.sweep.paired {2} else {1},
            parameter: study.sweep.parameter,
            values,
            samples: study.sweep.samples,
            interval_s: study.sweep.interval_hours * 3600.,
            secondary: if study.sweep.paired {
                let levels=parse_levels(&study.sweep.secondary_values)?;
                Some(cosmogon_sim::habitat_factorial::Secondary {parameter:study.sweep.secondary_parameter,values:levels.try_into().map_err(|_|"Second setting needs exactly two levels")?})
            } else {None},
            authored_relative_band: band,
        };
        p.validate()?;
        Ok(p)
    });
    if let Some(note)=plan.as_ref().ok().and_then(Plan::configuration_assumptions) {assumptions(ui,&note);}
    let validation = if study.use_saved_origin && study.origin.is_some() {plan.as_ref().map_err(Clone::clone).and_then(|p|study.saved_plan_validation(p))}else{plan.as_ref().map_err(Clone::clone).and_then(|_| {
        habitat_id
            .filter(|id| world.habitats.iter().any(|h| h.id == *id))
            .ok_or_else(|| "Select a live habitat".to_string())
            .and_then(|id| plan.as_ref().map_err(Clone::clone)?.validate_world(world, id))
    })};
    if let Err(e) = &validation {
        ui.colored_label(ACCENT, e);
    }
    if reference {
        ui.colored_label(ACCENT, "Clone this reference world to a sandbox first.");
    }
    if ui
        .add_enabled(
            !study.pending()
                && !reference
                && validation.is_ok()
                && !study.sweep.output.trim().is_empty(),
            egui::Button::new("Run and open first/last trajectories"),
        )
        .clicked()
    {
        if study.use_saved_origin && study.origin.is_some() {
            if let Err(e)=study.start_saved_sweep(plan.expect("validated plan"),study.sweep.output.clone()){study.status=Some(e);}
        }else{
            study.start_sweep(world.clone(),habitat_id.expect("validated habitat"),plan.expect("validated plan"),study.sweep.output.clone(),None);
        }
    }
    if let Some(manifest) = &study.sweep_manifest {
        ui.label(format!(
            "{} completed branches · {} ({})",
            manifest["runs"].as_array().map(Vec::len).unwrap_or(0),
            manifest["parameter"],
            manifest["unit"]
        ));
        if ui.button("Copy completed sweep manifest").clicked() {
            ui.ctx()
                .copy_text(serde_json::to_string_pretty(manifest).unwrap_or_default());
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saved_and_live_workers_capture_independent_origins_and_preserve_results_on_failure() {
        AsyncComputeTaskPool::get_or_init(bevy::tasks::TaskPool::new);
        let mut world=Universe::new(cosmogon_sim::UniverseSettings{scenario:cosmogon_sim::Scenario::SolarSystemLab,..Default::default()});
        let mut d=cosmogon_sim::crop_process::definition_at(cosmogon_sim::BodyRef{system:0,body:3},0.,0.);d.config.crew=0;
        world.apply_edit(cosmogon_sim::sandbox::Edit::CreateHabitat{definition:Box::new(d)}).unwrap();
        world.advance_to(world.time+86410.25,None,None);
        let root=std::env::temp_dir().join(format!("cosmogon-native-origins-{}",std::process::id()));let _=std::fs::remove_dir_all(&root);
        let plan=Plan::linear(Parameter::FirmPowerW,0.,10000.,3,3,60.).unwrap();
        habitat_sweep::execute(&world,0,&plan,&root.join("source")).unwrap();
        let mut study=Study{keep_experiment_at_launch:true,requested_branches:Some([1,2]),..Default::default()};
        study.commit_sweep(open_design(root.join("source"),root.join("source").to_string_lossy().into_owned(),Some(1)).unwrap()).unwrap();
        assert!(study.use_saved_origin);assert!(study.saved_plan_validation(&plan).is_ok());
        assert_eq!(study.origin_validation.as_ref().unwrap().0,plan);
        let original=study.metadata().unwrap()["single_design"].clone();
        let mut live=world.clone();live.advance_to(live.time+86400.,None,None);
        let live_fingerprint=cosmogon_sim::research::state_fingerprint(&live).unwrap();
        study.rerun_output=Some(root.join("saved").to_string_lossy().into_owned());
        study.launch_if_ready(&live,None,false);
        assert!(study.sweep_task.is_some());assert_eq!(study.metadata().unwrap()["single_design"],original);
        let request=study.sweep_request.clone().unwrap();
        study.path="unopened source".into();study.sweep.values="12, 13".into();
        let deadline=std::time::Instant::now()+std::time::Duration::from_secs(30);
        while study.sweep_task.is_some(){study.poll();assert!(std::time::Instant::now()<deadline);std::thread::sleep(std::time::Duration::from_millis(5));}
        assert_eq!(study.branches,[1,2]);assert_eq!(study.selected,1);assert!(study.experiment_mode);assert!(study.use_saved_origin);
        assert_eq!(study.sweep_request.as_ref(),Some(&request));assert_eq!(study.origin.as_ref().unwrap().cached_world().time,world.time);
        assert_eq!(study.single.as_ref().unwrap().recordings[1].rows,cosmogon_sim::habitat_design::read_single(&root.join("source")).unwrap().recordings[1].rows);
        assert_eq!(cosmogon_sim::research::state_fingerprint(&live).unwrap(),live_fingerprint);
        study.use_saved_origin=false;study.rerun_output=Some(root.join("live").to_string_lossy().into_owned());study.launch_if_ready(&live,None,false);
        while study.sweep_task.is_some(){study.poll();assert!(std::time::Instant::now()<deadline);std::thread::sleep(std::time::Duration::from_millis(5));}
        assert!(!study.use_saved_origin);assert_eq!(study.origin.as_ref().unwrap().fingerprint,live_fingerprint);
        assert_eq!(study.sweep_request.as_ref().unwrap()["origin"]["state_fingerprint"],live_fingerprint);
        assert_ne!(study.recording.as_ref().unwrap().rows[0].simulation_time_s,world.time);
        let before=study.metadata().unwrap()["single_design"].clone();
        let mut bad=plan.clone();bad.values=vec![-1.,50.];study.factorial_mode=true;
        study.start_sweep(live.clone(),0,bad,root.join("bad").to_string_lossy().into_owned(),None);
        assert!(study.factorial_mode);
        while study.sweep_task.is_some(){study.poll();assert!(std::time::Instant::now()<deadline);std::thread::sleep(std::time::Duration::from_millis(5));}
        assert!(study.factorial_mode);assert_eq!(study.metadata().unwrap()["single_design"],before);assert!(!root.join("bad").exists());
        let mut altered=live.clone();altered.habitats[0].state.potable_water_kg-=1.;altered.habitats[0].state.wastewater_kg+=1.;
        std::fs::write(root.join("live/base.cosmo"),cosmogon_sim::save::to_json(&altered,"changed").unwrap()).unwrap();
        study.use_saved_origin=true;study.start_saved_sweep(plan,root.join("changed").to_string_lossy().into_owned()).unwrap();
        study.figure.output=root.join("stale-figure").to_string_lossy().into_owned();study.figure.launch=true;
        study.balance.launch=true;study.replay.launch=true;
        while study.sweep_task.is_some(){study.poll();assert!(std::time::Instant::now()<deadline);std::thread::sleep(std::time::Duration::from_millis(5));}
        assert!(!study.figure.pending());assert!(!study.balance.pending());assert!(!study.replay.pending());assert!(!root.join("stale-figure").exists());
        assert!(study.status.as_ref().unwrap().contains("changed"));assert_eq!(study.metadata().unwrap()["single_design"],before);assert!(!root.join("changed").exists());
        study.commit(study.recording.clone().unwrap(),"ordinary".into(),Some(0)).unwrap();assert!(!study.use_saved_origin);assert!(study.origin.is_none());
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn actual_crop_figure_and_metadata_keep_loaded_source_when_drafts_change() {
        let d=cosmogon_sim::crop_process::definition_at(cosmogon_sim::BodyRef{system:0,body:0},0.,0.);
        let mut h=cosmogon_sim::habitat::Habitat::new(0,d,1000.).unwrap();
        let first=cosmogon_sim::habitat::Sample{simulation_time_s:1000.,habitat:h.clone()};
        h.step(1060.,cosmogon_sim::habitat::Environment{irradiance_w_m2:Some(0.),exterior_temperature_k:Some(210.),exterior_dose_rate_msv_day:None}).unwrap();
        let r=Recording{rows:vec![first,cosmogon_sim::habitat::Sample{simulation_time_s:1060.,habitat:h}],summary:serde_json::json!({"source":"actual installed crop"})};
        let mut study=Study{figure_mode:true,metric:Metric::CropProgress,..Default::default()};
        study.commit(r.clone(),"loaded crop folder".into(),Some(1)).unwrap();study.path="unopened crop folder".into();
        let f=study.prepare_figure().unwrap();assert_eq!(f.exact_samples,vec![r.rows]);assert_eq!(f.series[0].points[1].value,Some(0.));
        assert_eq!(f.crop_trajectory_context.as_ref().unwrap()["series"][0]["observations"][1]["retained_crop"]["current_crop_configuration"]["assume_maintained_conditions"],false);
        let m=study.metadata().unwrap();assert_eq!(m["selected_value"],0.);assert_eq!(m["crop_trajectory_context"]["series"][0]["observations"][1]["retained_crop"]["origin_world_time_s"],1000.);
        study.metric=Metric::HeatCropVapor;assert!(study.prepare_figure().unwrap().crop_trajectory_context.is_some());
        study.metric=Metric::Water;assert!(study.metadata().unwrap().get("crop_trajectory_context").is_none());
    }
    #[test]
    fn selected_heat_figure_keeps_loaded_coverage_and_unopened_path_separate() {
        let mut h=cosmogon_sim::habitat::Habitat::new(0,cosmogon_sim::habitat::Definition::at(cosmogon_sim::BodyRef {system:0,body:0},0.,0.),1000.).unwrap();
        let first=cosmogon_sim::habitat::Sample {simulation_time_s:1000.,habitat:h.clone()};
        h.step(1060.,cosmogon_sim::habitat::Environment::default()).unwrap();
        let r=Recording {rows:vec![first,cosmogon_sim::habitat::Sample {simulation_time_s:1060.,habitat:h}],summary:serde_json::json!({"source":"actual loaded heat"})};
        let mut study=Study {figure_mode:true,metric:Metric::HeatExcluded,..Default::default()};
        study.commit(r.clone(),"loaded heat folder".into(),Some(1)).unwrap();study.path="unopened folder".into();
        let f=study.prepare_figure().unwrap();assert_eq!(f.exact_samples,vec![r.rows]);assert_eq!(f.series[0].points[1].value,Some(60.));assert_eq!(f.series[0].points[1].support,"available_executed_subset");
        let m=study.metadata().unwrap();assert_eq!(m["selected_value"],60.);assert_eq!(m["heat_trajectory_context"]["series"][0]["observations"][1]["coverage"]["environment_missing_seconds"],60.);
    }
    #[test]
    fn balance_uses_actual_loaded_inputs_and_invalid_ranges_preserve_view() {
        let h=cosmogon_sim::habitat::Habitat::new(0,cosmogon_sim::habitat::Definition::at(cosmogon_sim::BodyRef{system:0,body:0},0.,0.),1000.).unwrap();let r=Recording{rows:vec![cosmogon_sim::habitat::Sample{simulation_time_s:1000.,habitat:h}],summary:serde_json::json!({"source":"actual"})};let mut study=Study{balance_mode:true,..Default::default()};study.commit(r.clone(),"actual-loaded".into(),None).unwrap();study.path="unopened-editable-path".into();
        let b=study.prepare_balance().unwrap();assert_eq!(b.reports[0].initial,r.rows[0]);assert_eq!(b.reports[0].final_sample,*r.rows.last().unwrap());study.balance.set(Ok(b.clone()));
        let first=study.metadata().unwrap();assert_eq!(first["balance"]["report"]["reports"][0]["recording"],r.summary);assert!(!first["paired_results_visible"].as_bool().unwrap());
        study.balance.from=r.rows.len();assert!(study.prepare_balance().is_err());assert_eq!(study.recording.as_ref().unwrap().rows,r.rows);
        study.balance.launch=true;study.recording=None;study.balance.reset();study.poll();assert!(!study.balance.pending());
    }
    #[test]
    fn figure_request_freezes_actual_recording_layout_and_never_uses_unopened_paths() {
        let h=cosmogon_sim::habitat::Habitat::new(0,cosmogon_sim::habitat::Definition::at(cosmogon_sim::BodyRef {system:0,body:0},0.,0.),1000.).unwrap();
        let r=Recording {rows:vec![cosmogon_sim::habitat::Sample {simulation_time_s:1000.,habitat:h}],summary:serde_json::json!({"source":"loaded immutable recording"})};
        let mut study=Study {figure_mode:true,metric:Metric::Temperature,..Default::default()};study.commit(r.clone(),"loaded-folder".into(),None).unwrap();study.metric=Metric::Temperature;study.path="unopened-folder".into();study.graph.layout.scale=super::super::habitat_graph::Scale::SignedLog;
        let figure=study.prepare_figure().unwrap();assert_eq!(figure.source_summaries,vec![r.summary]);assert_eq!(figure.exact_samples,vec![r.rows]);assert_eq!(figure.layout.metric,"temperature");assert_eq!(figure.layout.scale,super::super::habitat_graph::Scale::SignedLog);assert!(!study.metadata().unwrap()["paired_results_visible"].as_bool().unwrap());
        study.graph.layout.scale=super::super::habitat_graph::Scale::Linear;study.recording.as_mut().unwrap().rows[0].habitat.state.temperature_k+=5.;assert_ne!(figure.exact_samples[0],study.recording.as_ref().unwrap().rows);assert_eq!(figure.layout.scale,super::super::habitat_graph::Scale::SignedLog);
        study.figure.launch=true;study.recording=None;study.poll();assert!(!study.figure.pending());assert!(study.figure.metadata()["status"].as_str().unwrap().contains("Open a recording"));
    }
    #[test]
    fn invalid_selection_preserves_recording_and_recorded_values_are_independent() {
        let u = cosmogon_sim::Universe::new(cosmogon_sim::UniverseSettings {
            scenario: cosmogon_sim::Scenario::SolarSystemLab,
            ..Default::default()
        });
        let r = cosmogon_sim::BodyRef { system: 0, body: 3 };
        let h = cosmogon_sim::habitat::Habitat::new(
            0,
            cosmogon_sim::habitat::Definition::at(r, 0.0, 0.0),
            u.time,
        )
        .unwrap();
        let recording = Recording {
            rows: vec![cosmogon_sim::habitat::Sample {
                simulation_time_s: u.time,
                habitat: h,
            }],
            summary: serde_json::json!({"name":"Recorded"}),
        };
        let mut study = Study::default();
        study
            .commit(recording.clone(), "recorded".into(), None)
            .unwrap();
        study.pressure_mode=true;
        let before = study.metadata();
        assert_eq!(before.as_ref().unwrap()["pressure"]["loaded_source"],"recorded");
        study.path="different-unopened-folder".into();
        assert_eq!(study.metadata(),before);
        assert!(study.commit(recording, "bad".into(), Some(1)).is_err());
        assert_eq!(study.metadata(), before);
        assert!(u.habitats.is_empty());
        assert_eq!(study.metadata().unwrap()["selected_value"], 200.0);
    }
    #[test]
    fn failed_sweep_commit_and_reference_launch_preserve_prior_views() {
        let u = Universe::new(cosmogon_sim::UniverseSettings {
            scenario: cosmogon_sim::Scenario::SolarSystemLab,
            ..Default::default()
        });
        let h = cosmogon_sim::habitat::Habitat::new(
            0,
            cosmogon_sim::habitat::Definition::at(
                cosmogon_sim::BodyRef { system: 0, body: 3 },
                0.,
                0.,
            ),
            u.time,
        )
        .unwrap();
        let r = Recording {
            rows: vec![cosmogon_sim::habitat::Sample {
                simulation_time_s: u.time,
                habitat: h,
            }],
            summary: serde_json::json!({"name":"Prior"}),
        };
        let mut study = Study::default();
        study.commit(r.clone(), "prior".into(), None).unwrap();
        study.comparison = Some(r.clone());
        let before = study.metadata();
        let make = |sample, manifest| SweepResult {
            a: r.clone(),
            b: r.clone(),
            manifest,
            paired:None,
            single:None,
            origin:None,
            origin_selection:None,
            path: "new".into(),
            sample,
        };
        assert!(study
            .commit_sweep(make(Some(1), serde_json::json!({"runs":[{},{}]})))
            .is_err());
        assert_eq!(study.metadata(), before);
        assert_eq!(study.path, "prior");
        assert!(study
            .commit_sweep(make(None, serde_json::json!({"runs":[]})))
            .is_err());
        assert_eq!(study.metadata(), before);
        assert_eq!(study.path, "prior");
        study.launch = Some(Launch {
            plan: Plan::linear(Parameter::FirmPowerW, 0., 4000., 2, 1, 60.).unwrap(),
            id: Some(0),
            output: "unused".into(),
            sample: None,
            initial_path: None,
        });
        study.launch_if_ready(&u, None, true);
        assert!(!study.pending());
        assert_eq!(study.metadata(), before);
        assert!(u.habitats.is_empty());
        study
            .commit_sweep(make(None, serde_json::json!({"runs":[{},{}]})))
            .unwrap();
        assert_eq!(study.path, "new/run-000");
        assert_eq!(study.comparison_path, "new/run-001");
        assert!(study.metadata().unwrap()["comparison"]["selected_pair"]["b_minus_a"] == 0.);
    }
    #[test]
    fn moisture_worker_opens_actual_comparison_and_preserves_view_on_invalid_branch() {
        let mut world = Universe::new(cosmogon_sim::UniverseSettings {
            scenario: cosmogon_sim::Scenario::SolarSystemLab,
            ..Default::default()
        });
        let mut definition = cosmogon_sim::habitat::Definition::at(cosmogon_sim::BodyRef {system:0,body:3},0.0,0.0);
        definition.config.moisture = Some(cosmogon_sim::moisture::Config::default());
        world.apply_edit(cosmogon_sim::sandbox::Edit::CreateHabitat {definition:Box::new(definition)}).unwrap();
        world.advance_to(world.time+86400.0,None,None);
        let before = cosmogon_sim::research::state_fingerprint(&world).unwrap();
        let root = std::env::temp_dir().join(format!("cosmogon-native-moisture-worker-{}",std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let plan = Plan::linear(Parameter::MoisturePowerW,0.0,500.0,2,3,3600.0).unwrap();
        let result = run_sweep(world.clone(),0,plan,root.join("good").to_string_lossy().into_owned(),Some(2)).unwrap();
        let mut study = Study {metric:Metric::Vapor,..Default::default()};
        study.commit_sweep(result).unwrap();
        let metadata = study.metadata().unwrap();
        assert_eq!(metadata["sweep"]["parameter"],"moisture_power_w");
        let pair = &metadata["comparison"]["selected_pair"];
        assert!(pair["b_minus_a"].as_f64().unwrap() < 0.0);
        assert_eq!(pair["a_value"].as_f64(),Some(study.recording.as_ref().unwrap().rows[2].habitat.state.water_vapor_kg));
        let plan = Plan::linear(Parameter::MoistureCondensateCapacityKg,20.0,0.0,2,3,60.0).unwrap();
        assert!(run_sweep(world.clone(),0,plan,root.join("invalid").to_string_lossy().into_owned(),None).is_err());
        assert!(!root.join("invalid").exists());
        assert_eq!(study.metadata(),Some(metadata));
        assert_eq!(cosmogon_sim::research::state_fingerprint(&world).unwrap(),before);
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn paired_worker_reopens_verified_results_and_preserves_views_on_bad_input() {
        let mut world=Universe::new(cosmogon_sim::UniverseSettings {scenario:cosmogon_sim::Scenario::SolarSystemLab,..Default::default()});
        let mut definition=cosmogon_sim::habitat::Definition::at(cosmogon_sim::BodyRef {system:0,body:3},0.0,0.0);
        definition.config.moisture=Some(cosmogon_sim::moisture::Config::default());
        world.apply_edit(cosmogon_sim::sandbox::Edit::CreateHabitat {definition:Box::new(definition)}).unwrap();world.advance_to(world.time+86400.0,None,None);
        let root=std::env::temp_dir().join(format!("cosmogon-native-paired-worker-{}",std::process::id()));let _=std::fs::remove_dir_all(&root);
        let mut plan=Plan::linear(Parameter::MoisturePowerW,0.0,500.0,2,3,3600.0).unwrap();plan.schema_version=2;plan.secondary=Some(cosmogon_sim::habitat_factorial::Secondary {parameter:Parameter::MoistureAirborneFraction,values:[0.2,1.0]});
        let result=run_sweep(world,0,plan,root.to_string_lossy().into_owned(),Some(2)).unwrap();
        let mut study=Study {factorial_mode:true,metric:Metric::Vapor,..Default::default()};study.commit_sweep(result).unwrap();
        let selected=study.metadata().unwrap()["paired_selected_analysis"].clone();
        study.selected=1;let earlier=study.metadata().unwrap()["paired_selected_analysis"].clone();assert_eq!(earlier["observation_index"],1);assert_ne!(earlier,selected);
        assert_eq!(study.paired_curve.as_ref().unwrap().1.len(),3);
        let checked_source=study.paired_source.clone();let view_path=study.path.clone();study.path="different-unopened-folder".into();assert_eq!(study.paired_source,checked_source);study.path=view_path;
        study.selected=2;study.graph.layout.window=[0.2,0.8];study.graph.layout.scale=super::super::habitat_graph::Scale::SignedLog;
        let view=study.graph_metadata();assert_eq!(view["layout"]["metric"],"vapor");assert_eq!(view["plot"]["displayed"][0].as_array().unwrap().len(),1);assert_eq!(view["plot"]["displayed"][0][0]["index"],1);assert_eq!(view["selected_observation_visible"],false);assert_eq!(study.metadata().unwrap()["paired_selected_analysis"],selected);
        let before=study.metadata();assert!(before.as_ref().unwrap()["paired_results_visible"].as_bool().unwrap());assert_eq!(before.as_ref().unwrap()["sweep"]["runs"].as_array().unwrap().len(),4);
        let reopened=open_design(root.clone(),root.to_string_lossy().into_owned(),Some(2)).unwrap();study.commit_sweep(reopened).unwrap();assert_eq!(study.metadata(),before);
        assert!(check_input(&root,Some(3)).is_err());
        std::fs::write(root.join("factorial.json"),b"{}").unwrap();assert!(open_design(root.clone(),root.to_string_lossy().into_owned(),None).is_err());assert_eq!(study.metadata(),before);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn geometry_worker_restores_exact_staged_plan_and_preserves_loaded_evidence() {
        let mut u=Universe::new(cosmogon_sim::UniverseSettings {scenario:cosmogon_sim::Scenario::SolarSystemLab,..Default::default()});
        u.apply_edit(cosmogon_sim::sandbox::Edit::CreateHabitat {definition:Box::new(cosmogon_sim::habitat::Definition::at(cosmogon_sim::BodyRef {system:0,body:3},0.,0.))}).unwrap();
        let root=std::env::temp_dir().join(format!("cosmogon-native-geometry-{}",std::process::id()));let _=std::fs::remove_dir_all(&root);
        let mut p=Plan::linear(Parameter::VolumeM3,250.,1000.,2,3,3600.).unwrap();
        p.schema_version=2;p.secondary=Some(cosmogon_sim::habitat_factorial::Secondary {parameter:Parameter::HeatCapacityJK,values:[5e7,1e9]});
        let result=run_sweep(u.clone(),0,p.clone(),root.join("good").to_string_lossy().into_owned(),Some(2)).unwrap();
        let mut study=Study {keep_experiment_at_launch:true,..Default::default()};study.commit_sweep(result).unwrap();
        assert!(study.experiment_mode);assert!(!study.keep_experiment_at_launch);
        assert_eq!(study.sweep.parameter,p.parameter);assert_eq!(parse_levels(&study.sweep.values).unwrap(),p.values);
        assert_eq!(study.sweep.secondary_parameter,Parameter::HeatCapacityJK);assert!(study.sweep.output.is_empty());
        let before=study.metadata();assert_eq!(before.as_ref().unwrap()["paired_selected_analysis"]["metrics"].as_object().unwrap().len(),19);
        let invalid=Plan::linear(Parameter::VolumeM3,250.,0.,2,3,3600.).unwrap();
        assert!(run_sweep(u,0,invalid,root.join("bad").to_string_lossy().into_owned(),None).is_err());
        assert_eq!(study.metadata(),before);assert!(!root.join("bad").exists());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn crop_worker_restores_typed_controls_and_keeps_completed_inputs_separate() {
        let mut u=Universe::new(cosmogon_sim::UniverseSettings {scenario:cosmogon_sim::Scenario::SolarSystemLab,..Default::default()});
        let mut d=cosmogon_sim::crop_process::definition_at(cosmogon_sim::BodyRef {system:0,body:3},0.,0.);
        d.config.crew=0;d.config.solar_area_m2=0.;d.config.firm_power_w=5000.;d.initial.battery_j=0.;
        u.apply_edit(cosmogon_sim::sandbox::Edit::CreateHabitat {definition:Box::new(d)}).unwrap();
        let root=std::env::temp_dir().join(format!("cosmogon-native-crop-controls-{}",std::process::id()));let _=std::fs::remove_dir_all(&root);
        let mut p=Plan::linear(Parameter::CropMaintainedConditions,0.,1.,2,3,3600.).unwrap();p.schema_version=2;
        p.secondary=Some(cosmogon_sim::habitat_factorial::Secondary {parameter:Parameter::CropAuxiliaryPowerW,values:[0.,10000.]});
        let result=run_sweep(u.clone(),0,p.clone(),root.join("good").to_string_lossy().into_owned(),Some(2)).unwrap();
        let mut study=Study {keep_experiment_at_launch:true,..Default::default()};study.commit_sweep(result).unwrap();
        assert!(study.experiment_mode);assert_eq!(study.sweep.parameter,p.parameter);assert_eq!(parse_levels(&study.sweep.values).unwrap(),p.values);
        assert_eq!(study.sweep.secondary_parameter,Parameter::CropAuxiliaryPowerW);assert!(study.sweep.output.is_empty());
        study.metric=Metric::CropProgress;
        let completed=study.metadata().unwrap()["paired_selected_analysis"].clone();
        assert!(completed["metrics"].get("crop_progress").is_some());assert!(completed["metrics"].get("total_pressure").is_none());
        assert_eq!(study.metadata().unwrap()["staged_crop_plan"]["parameter"],"crop_maintained_conditions");
        study.sweep.values="1, 0".into();study.path="unopened-other-folder".into();
        assert_eq!(study.metadata().unwrap()["paired_selected_analysis"],completed);
        study.factorial_mode=true;
        assert_eq!(study.prepare_figure().unwrap().crop_trajectory_context.as_ref().unwrap()["series"].as_array().unwrap().len(),4);
        let invalid=Plan::linear(Parameter::CropAuxiliaryPowerW,-1.,50.,2,3,3600.).unwrap();
        assert!(run_sweep(u,0,invalid,root.join("bad").to_string_lossy().into_owned(),None).is_err());assert!(!root.join("bad").exists());
        assert_eq!(study.metadata().unwrap()["paired_selected_analysis"],completed);std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn saved_single_worker_retains_all_branches_controls_and_source_independent_of_drafts() {
        let mut u=Universe::new(cosmogon_sim::UniverseSettings{scenario:cosmogon_sim::Scenario::SolarSystemLab,..Default::default()});
        u.apply_edit(cosmogon_sim::sandbox::Edit::CreateHabitat{definition:Box::new(cosmogon_sim::habitat::Definition::at(cosmogon_sim::BodyRef{system:0,body:3},0.,0.))}).unwrap();
        u.advance_to(u.time+86410.25,None,None);
        let root=std::env::temp_dir().join(format!("cosmogon-native-single-{}",std::process::id()));let _=std::fs::remove_dir_all(&root);
        let p=Plan::linear(Parameter::VolumeM3,250.,1000.,3,3,3600.).unwrap();
        habitat_sweep::execute(&u,0,&p,&root).unwrap();
        let mut study=Study{requested_branches:Some([1,2]),keep_experiment_at_launch:true,..Default::default()};
        study.commit_sweep(open_design(root.clone(),root.to_string_lossy().into_owned(),Some(1)).unwrap()).unwrap();
        assert!(!study.factorial_mode);assert!(study.paired.is_none());assert_eq!(study.single.as_ref().unwrap().recordings.len(),3);
        assert_eq!(study.branches,[1,2]);assert!(study.sweep.output.is_empty());assert_eq!(parse_levels(&study.sweep.values).unwrap(),p.values);
        let original=study.metadata().unwrap()["single_design"].clone();study.path="unopened".into();study.sweep.values="12, 13".into();
        assert_eq!(study.metadata().unwrap()["single_design"],original);
        assert_eq!(study.prepare_figure().unwrap().single_design_context.unwrap()["selected_branches"],serde_json::json!([1,2]));
        study.select_branches([2,0]).unwrap();assert_eq!(study.recording.as_ref().unwrap().rows,study.single.as_ref().unwrap().recordings[2].rows);
        let before=study.metadata();assert!(study.select_branches([3,0]).is_err());assert_eq!(study.metadata(),before);
        let mut m:serde_json::Value=serde_json::from_slice(&std::fs::read(root.join("manifest.json")).unwrap()).unwrap();m["complete"]=false.into();std::fs::write(root.join("manifest.json"),serde_json::to_vec(&m).unwrap()).unwrap();
        assert!(open_design(root.clone(),root.to_string_lossy().into_owned(),None).is_err());assert_eq!(study.metadata(),before);
        assert!(check_input(&root,Some(0)).is_err());std::fs::remove_dir_all(root).unwrap();
    }

}
