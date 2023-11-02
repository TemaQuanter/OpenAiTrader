#!/bin/bash

# This function loads environmental variables from a file.
load_env() {
    local env_file="$1"

    if [[ ! -f "$env_file" ]]; then
        echo "File not found: $env_file"
        return 1
    fi

    while IFS='=' read -r key temp || [[ -n "$key" ]]; do
        # Ignore lines starting with #
        if [[ $key =~ ^# ]]; then
            continue
        fi

        # If there's no '=', assume it's not a key-value pair and skip
        if [[ -z $temp ]]; then
            continue
        fi

        value=$(echo "$temp" | sed -e 's/^[ \t]*//' -e 's/[ \t]*$//')
        if [[ "$value" =~ ^\".*\"$ ]]; then
            value="${value:1:${#value}-2}"
        fi
            export "$key=$value"
    done < "$env_file"
} # end load_env
