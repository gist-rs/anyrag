#!/bin/bash

# --- Configuration ---
URL="https://news.smol.ai/rss.xml"
ETAG_FILE="$HOME/.smol_rss_etag" # Hidden file to store the ETag

# --- Main Logic ---

# Check if we have a stored ETag from a previous run
if [ ! -f "$ETAG_FILE" ]; then
    # This is the first run
    echo "🔍 First run. Fetching and saving the current ETag..."

    # Get the ETag from the headers, grep for the 'etag' line (case-insensitive),
    # and use cut to extract the value inside the quotes.
    INITIAL_ETAG=$(curl -sI "$URL" | grep -i '^etag:' | cut -d '"' -f 2)

    if [ -n "$INITIAL_ETAG" ]; then
        echo "$INITIAL_ETAG" > "$ETAG_FILE"
        echo "✅ Initial ETag saved to $ETAG_FILE"
    else
        echo "🚨 Error: Could not retrieve the initial ETag from the server."
        exit 1
    fi
else
    # This is a subsequent run
    STORED_ETAG=$(cat "$ETAG_FILE")
    echo "Checking for updates against stored ETag: $STORED_ETAG"

    # Perform a conditional request using the stored ETag.
    # We only want the HTTP status code as output.
    HTTP_STATUS=$(curl -s -o /dev/null -w "%{http_code}" -H "If-None-Match: \"$STORED_ETAG\"" "$URL")

    case "$HTTP_STATUS" in
        "200")
            echo "🎉 Feed has been UPDATED!"
            echo "Fetching and saving the new ETag..."

            # Since it was updated, get the NEW ETag from the response headers and save it for next time.
            # The -D - flag dumps response headers to stdout.
            NEW_ETAG=$(curl -s -D - -o /dev/null "$URL" | grep -i '^etag:' | cut -d '"' -f 2)
            echo "$NEW_ETAG" > "$ETAG_FILE"
            echo "✅ New ETag saved."
            ;;
        "304")
            echo "👍 Feed is UNCHANGED. No new updates."
            ;;
        *)
            echo "🚨 Error: Received an unexpected HTTP status code: $HTTP_STATUS"
            ;;
    esac
fi
