#!/bin/sh

set -e

SCRIPT_PATH=$(readlink -f $0)
SCRIPT_DIR=$(dirname "$SCRIPT_PATH")
ROOT_DIR=$(readlink -f "$SCRIPT_DIR/../..")

SERVER_URI="http://localhost:3000"

SERVER_DIR="$ROOT_DIR/server"
PKI_DIR="$SERVER_DIR/pki"
EXAMPLES_DIR="$SERVER_DIR/examples"

FD_ID="$PKI_DIR/fd_id"
IDP_ID="$PKI_DIR/idp_id"
QES_ID="$PKI_DIR/qes_id"
QES_CRT="$PKI_DIR/qes.crt"

function create_access_token() {
    cargo run \
        --manifest-path="$ROOT_DIR/Cargo.toml" \
        --quiet \
        -p tool \
        create-access-token \
            --key "$IDP_ID"
}

function create_task() {
    echo "Create Task..."

    local -n RESULT=$1
    local OUTPUT=$(curl \
        --silent \
        --data-binary "@$EXAMPLES_DIR/task_create_parameters.json" \
        --header "Content-Type: application/fhir+json" \
        --header "Authorization: Bearer $ACCESS_TOKEN_ARTZ" \
        "$SERVER_URI/Task/\$create")
    local TASK_ID=$(echo "$OUTPUT" \
        | grep -Po '"id":\K.*?[^\\]",' \
        | grep -Po '[A-Za-z0-9_-]+')
    local ACCESS_CODE=$(echo "$OUTPUT" \
        | grep -Po '{"system":"https://gematik.de/fhir/Namingsystem/AccessCode","value":\K"[A-Za-z0-9]+"}' \
        | grep -Po '[A-Za-z0-9_-]+')

    echo "    ID:           $TASK_ID"
    echo "    ACCESS_CODE:  $ACCESS_CODE"
    echo "    Done"

    RESULT=("$TASK_ID" "$ACCESS_CODE")
}

function activate_task() {
    local TASK_ID="$1"
    local ACCESS_CODE="$2"
    local KVNR="$3"

    echo "Activate Task..."
    echo "    KVNR:         $KVNR"

    local KBV_BUNDLE_ID=$(xxd -u -l 32 -c 32 -p /dev/urandom)
    local KBV_BUNDLE_ID="<id value=\"$KBV_BUNDLE_ID\"\\/>"
    local KVNR="<value value=\"$KVNR\"\\/>"
    local KBV_BUNDLE=$(cat "$EXAMPLES_DIR/kbv_bundle.xml" \
        | sed "s/<id value=\"281a985c-f25b-4aae-91a6-41ad744080b0\"\\/>/$KBV_BUNDLE_ID/g" \
        | sed "s/<value value=\"X234567890\"\\/>/$KVNR/g")

    local QES_DATA=$(echo "$KBV_BUNDLE" \
        | cargo run \
            --manifest-path="$ROOT_DIR/Cargo.toml" \
            --quiet \
            -p tool \
            pkcs7-sign \
                --key "$QES_ID" \
                --cert "$QES_CRT" \
        | sed 's/-----.*-----//g' \
        | sed ':a;N;$!ba;s/\n//g' \
        | sed '/^$/d')
    local QES_DATA="\"data\":\"$QES_DATA\""
    local QES_DATA=$(echo "$QES_DATA" \
        | sed \
            '/"data":".*"/{
                r /dev/stdin
                d
                }' \
            "$EXAMPLES_DIR/task_activate_parameters.json")

    echo "$QES_DATA" | curl \
        --fail \
        --silent \
        --data-binary @- \
        --output /dev/null \
        --header "Content-Type: application/fhir+json" \
        --header "Authorization: Bearer $ACCESS_TOKEN_ARTZ" \
        --header "X-AccessCode: $ACCESS_CODE" \
        "http://localhost:3000/Task/$TASK_ID/\$activate"

    echo "    Done"
}

function new_task() {
    KVNR="$1"

    create_task task
    activate_task "${task[0]}" "${task[1]}" "${KVNR}"
}

ACCESS_TOKEN_ARTZ=$(cat "$EXAMPLES_DIR/claims_arzt.json" | create_access_token)

new_task "X234567890"
new_task "X234567890"
new_task "X234567890"
new_task "X234567890"

new_task "X987654321"
new_task "X987654321"
new_task "X987654321"

new_task "X111111111"
new_task "X111111111"

new_task "X222222222"
