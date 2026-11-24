; Inno-Setup-Skript für den Windows-Installer von RMengine.
; Wird von .github/workflows/release.yml gebaut:
;   iscc /DAppVersion=0.1.0 installer\windows\rmengine.iss
;
; Installiert wird ohne Administratorrechte in "Dokumente\RMengine",
; damit man die Weltdaten bequem bearbeiten kann.
; Bei einem erneuten Installieren (Update) werden vorhandene Weltdaten
; NICHT überschrieben, und beim Deinstallieren bleiben sie erhalten.

#ifndef AppVersion
  #define AppVersion "0.0.0"
#endif
#define Root "..\.."

[Setup]
AppId={{730155F5-ACE6-4808-B2C0-789C7B0CFF72}
AppName=RMengine
AppVersion={#AppVersion}
AppVerName=RMengine {#AppVersion}
AppPublisherURL=https://github.com/areadox/RMengine
AppSupportURL=https://github.com/areadox/RMengine/issues
AppUpdatesURL=https://github.com/areadox/RMengine/releases
DefaultDirName={userdocs}\RMengine
DefaultGroupName=RMengine
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
OutputDir={#Root}\dist
OutputBaseFilename=RMengine-Setup-{#AppVersion}
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
UninstallDisplayIcon={app}\rmengine.exe
UninstallDisplayName=RMengine

[Languages]
Name: "german"; MessagesFile: "compiler:Languages\German.isl"

[Tasks]
Name: "desktopicon"; Description: "Verknüpfung auf dem Desktop anlegen"; Flags: unchecked

[Files]
Source: "{#Root}\dist\windows\rmengine.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#Root}\dist\windows\rmeditor.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#Root}\ANLEITUNG.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#Root}\README.md"; DestDir: "{app}"; Flags: ignoreversion
; Die Beispielwelt nur bei der ersten Installation kopieren. Gibt es schon eine Welt
; (daten\welt.toml), bleibt sie unberührt – auch neue Beispieldateien kommen nicht hinzu.
Source: "{#Root}\daten\*"; DestDir: "{app}\daten"; Flags: onlyifdoesntexist uninsneveruninstall recursesubdirs createallsubdirs; Check: BeispielweltNoetig

[Icons]
Name: "{group}\RMengine starten"; Filename: "{app}\rmengine.exe"; WorkingDir: "{app}"
Name: "{group}\Welteditor"; Filename: "{app}\rmeditor.exe"; Parameters: """{app}\daten"""; WorkingDir: "{app}"
Name: "{group}\Weltdaten bearbeiten (Ordner öffnen)"; Filename: "{app}\daten"
Name: "{group}\Weltdaten prüfen"; Filename: "{cmd}"; Parameters: "/k chcp 65001 >nul & ""{app}\rmengine.exe"" --pruefen"; WorkingDir: "{app}"
Name: "{group}\Nach Updates suchen"; Filename: "{cmd}"; Parameters: "/k chcp 65001 >nul & ""{app}\rmengine.exe"" --update"; WorkingDir: "{app}"
Name: "{group}\Anleitung"; Filename: "{win}\notepad.exe"; Parameters: """{app}\ANLEITUNG.md"""
Name: "{group}\RMengine deinstallieren"; Filename: "{uninstallexe}"
Name: "{userdesktop}\RMengine"; Filename: "{app}\rmengine.exe"; WorkingDir: "{app}"; Tasks: desktopicon
Name: "{userdesktop}\RMengine Welteditor"; Filename: "{app}\rmeditor.exe"; Parameters: """{app}\daten"""; WorkingDir: "{app}"; Tasks: desktopicon

[Run]
Filename: "{app}\rmengine.exe"; Description: "RMengine jetzt starten"; WorkingDir: "{app}"; Flags: postinstall nowait skipifsilent

[UninstallDelete]
Type: files; Name: "{app}\rmengine.exe.alt"
Type: files; Name: "{app}\rmengine.exe.neu"
Type: files; Name: "{app}\rmeditor.exe.alt"
Type: files; Name: "{app}\rmeditor.exe.neu"

[Code]
var
  WeltGeprueft: Boolean;
  WeltDa: Boolean;

{ Einmal vor dem Kopieren nachsehen, ob es schon eine Welt gibt (danach würde
  welt.toml ja durch das Kopieren selbst existieren). }
function BeispielweltNoetig: Boolean;
begin
  if not WeltGeprueft then
  begin
    WeltDa := FileExists(ExpandConstant('{app}\daten\welt.toml'));
    WeltGeprueft := True;
  end;
  Result := not WeltDa;
end;

