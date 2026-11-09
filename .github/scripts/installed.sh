#!/usr/bin/env bash
# Whether version $1 is the one installed by `packages.yml`: the AppImage in place is that
# version's file (Linux), or the per-user install says that version (Windows). An update under
# Windows finishes in an installer of its own: waited for, two minutes at most.
set -euo pipefail
expected=$1
case "$expected" in
  1.0.0) packages=old ;;
  1.0.1) packages=new ;;
  *) echo "::error::no packages for $expected"; exit 2 ;;
esac

if [ "$RUNNER_OS" = Linux ]; then
  installed="$RUNNER_TEMP/installed/DevNotes.AppImage"
  want=$(sha256sum "$packages"/*.AppImage | cut -d' ' -f1)
  have=$(sha256sum "$installed" | cut -d' ' -f1)
  if [ "$want" != "$have" ] || [ ! -x "$installed" ]; then
    echo "::error::the AppImage in place is not $expected"
    exit 1
  fi
  echo "installed: $expected"
  exit 0
fi

# The uninstall entry the NSIS installer writes under HKCU, a field of it.
entry() {
  pwsh -NoProfile -Command "(Get-ItemProperty 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*' -ErrorAction SilentlyContinue | Where-Object DisplayName -eq 'DevNotes').$1" | tr -d '\r"'
}
for _ in $(seq 1 120); do
  found=$(entry DisplayVersion)
  folder=$(entry InstallLocation)
  if [ "$found" = "$expected" ] && [ -f "$folder/devnotes.exe" ] && [ -f "$folder/identity/names.txt" ]; then
    echo "installed: $expected in $folder"
    exit 0
  fi
  sleep 1
done
echo "::error::installed: '${found:-nothing}', expected $expected"
exit 1
