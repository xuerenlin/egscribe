#!/bin/bash

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" >/dev/null 2>&1 && pwd)
SCRIPT_PATH="${SCRIPT_DIR}/$(basename "${BASH_SOURCE[0]}")"

TARGET_APP="egscribe"
DESKTOP_FILE="${SCRIPT_DIR}/${TARGET_APP}.desktop"
ICON_FILE="${SCRIPT_DIR}/${TARGET_APP}.png"
APP_FILE="${SCRIPT_DIR}/${TARGET_APP}"

if [ ! -f "${APP_FILE}" ]; then
    echo "Error:  ${APP_FILE} not found"
    exit 1
fi

if [ ! -f "${ICON_FILE}" ]; then
    echo "Error: ${ICON_FILE} not found"
    exit 1
fi

cat > "${DESKTOP_FILE}" << EOF
[Desktop Entry]
Name=EgScribe
Comment=Custom EgScribe Application
Exec="${APP_FILE}" %F
Path=${SCRIPT_DIR}
Icon=${ICON_FILE}
Terminal=false
Type=Application
Categories=Utility;TextEditor;
MimeType=text/plain;
EOF

chmod +x "${APP_FILE}"
chmod +x "${DESKTOP_FILE}"

echo "Create ${DESKTOP_FILE} finished."
echo "To add to the system, please execute:"
echo "  sudo cp \"${DESKTOP_FILE}\" /usr/share/applications/"
echo "  sudo update-desktop-database"
