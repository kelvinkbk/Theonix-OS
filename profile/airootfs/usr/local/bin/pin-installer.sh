#!/bin/bash
# Wait for Plasma shell to initialize
sleep 5

# Try both qdbus and qdbus6 depending on what is available
QDBUS="qdbus"
if command -v qdbus6 >/dev/null 2>&1; then
    QDBUS="qdbus6"
fi

$QDBUS org.kde.plasmashell /PlasmaShell org.kde.PlasmaShell.evaluateScript '
var ps = panels();
for (var i = 0; i < ps.length; i++) {
    var widgets = ps[i].widgets();
    for (var j = 0; j < widgets.length; j++) {
        if (widgets[j].type === "org.kde.plasma.icontasks") {
            var launchers = widgets[j].readConfig("launchers");
            if (launchers.indexOf("theonix-installer.desktop") === -1) {
                // Prepend theonix-installer.desktop to the launchers list
                widgets[j].writeConfig("launchers", "applications:theonix-installer.desktop," + launchers);
            }
        }
    }
}
'

# Remove the autostart entry so it doesn't run again
rm -f ~/.config/autostart/pin-installer.desktop
