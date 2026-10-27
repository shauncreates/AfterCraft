#ifndef AppVersion
  #define AppVersion "0.16.0.1"
#endif
#ifndef Payload
  #define Payload "..\..\artifacts\AfterCraft"
#endif

[Setup]
AppId={{81C0B7DA-6755-4E89-9655-CC91BC49117A}
AppName=AfterCraft
AppVersion={#AppVersion}
AppVerName=AfterCraft {#AppVersion} Preview
AppPublisher=AfterCraft contributors
AppPublisherURL=https://github.com/shauncreates/AfterCraft
AppSupportURL=https://github.com/shauncreates/AfterCraft/issues
AppUpdatesURL=https://github.com/shauncreates/AfterCraft/releases
DefaultDirName={localappdata}\Programs\AfterCraft
DefaultGroupName=AfterCraft
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0
DisableProgramGroupPage=yes
DisableDirPage=no
LicenseFile=..\..\LICENSE-MIT
OutputDir=..\..\artifacts\installers
OutputBaseFilename=AfterCraft-0.16.0-preview.1-windows-x64-Setup
SetupIconFile=..\..\assets\aftercraft\aftercraft.ico
UninstallDisplayIcon={app}\AfterCraft.exe
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
CloseApplications=yes
CloseApplicationsFilter=AfterCraft.exe,aftercraft-cli.exe
RestartApplications=no

[Tasks]
Name: "desktopicon"; Description: "Create a desktop shortcut"; GroupDescription: "Shortcuts:"; Flags: unchecked
Name: "openwith"; Description: "Add AfterCraft to Open with for compatible project files"; GroupDescription: "Windows integration:"; Flags: unchecked

[Files]
; Bytecode caches can contain absolute paths from the build machine.
Source: "{#Payload}\*"; DestDir: "{app}"; Excludes: "*.pyc,*.pyo"; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
Name: "{userprograms}\AfterCraft\AfterCraft"; Filename: "{app}\AfterCraft.exe"; WorkingDir: "{app}"
Name: "{userprograms}\AfterCraft\Uninstall AfterCraft"; Filename: "{uninstallexe}"
Name: "{userdesktop}\AfterCraft"; Filename: "{app}\AfterCraft.exe"; WorkingDir: "{app}"; Tasks: desktopicon

[Registry]
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\App Paths\AfterCraft.exe"; ValueType: string; ValueData: "{app}\AfterCraft.exe"; Flags: uninsdeletekey
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\App Paths\AfterCraft.exe"; ValueName: "Path"; ValueType: string; ValueData: "{app}"; Flags: uninsdeletekey
Root: HKCU; Subkey: "Software\Classes\AfterCraft.Project"; ValueType: string; ValueData: "AfterCraft compatible project"; Flags: uninsdeletekey; Tasks: openwith
Root: HKCU; Subkey: "Software\Classes\AfterCraft.Project\DefaultIcon"; ValueType: string; ValueData: "{app}\AfterCraft.exe,0"; Tasks: openwith
Root: HKCU; Subkey: "Software\Classes\AfterCraft.Project\shell\open\command"; ValueType: string; ValueData: """{app}\AfterCraft.exe"" ""%1"""; Tasks: openwith
Root: HKCU; Subkey: "Software\Classes\.ecproj\OpenWithProgids"; ValueName: "AfterCraft.Project"; ValueType: string; ValueData: ""; Flags: uninsdeletevalue; Tasks: openwith
Root: HKCU; Subkey: "Software\Classes\.ecprojx\OpenWithProgids"; ValueName: "AfterCraft.Project"; ValueType: string; ValueData: ""; Flags: uninsdeletevalue; Tasks: openwith

[Run]
Filename: "{app}\AfterCraft.exe"; Description: "Open AfterCraft"; Flags: nowait postinstall skipifsilent

; User data under APPDATA\AfterCraft is deliberately not listed for uninstallation.
