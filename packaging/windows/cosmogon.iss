; Inno Setup script for the Windows installer (built by .github/workflows/release.yml).
#define AppVersion GetEnv("COSMOGON_VERSION")

[Setup]
AppId={{6E1A8C2B-6B39-4E8B-9C1E-C05A0606C0D1}
AppName=Cosmogon
AppVersion={#AppVersion}
AppPublisher=Cosmogon contributors
DefaultDirName={autopf}\Cosmogon
DefaultGroupName=Cosmogon
OutputBaseFilename=Cosmogon-{#AppVersion}-Windows-Setup
SetupIconFile=..\icons\cosmogon.ico
UninstallDisplayIcon={app}\Cosmogon.exe
Compression=lzma2
SolidCompression=yes
ArchitecturesInstallIn64BitMode=x64compatible
OutputDir=..\..\dist

[Files]
Source: "..\..\target\release\cosmogon.exe"; DestDir: "{app}"; DestName: "Cosmogon.exe"

[Icons]
Name: "{group}\Cosmogon"; Filename: "{app}\Cosmogon.exe"
Name: "{autodesktop}\Cosmogon"; Filename: "{app}\Cosmogon.exe"; Tasks: desktopicon

[Tasks]
Name: "desktopicon"; Description: "Create a desktop shortcut"

[Run]
Filename: "{app}\Cosmogon.exe"; Description: "Launch Cosmogon"; Flags: nowait postinstall skipifsilent
