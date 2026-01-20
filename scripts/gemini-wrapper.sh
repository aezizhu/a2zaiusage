#!/bin/bash
# Gemini CLI Wrapper for a2zusage
# This script wraps the Gemini CLI to capture usage statistics
#
# Installation:
#   1. Make this script executable: chmod +x gemini-wrapper.sh
#   2. Add alias to your shell: alias gemini='/path/to/gemini-wrapper.sh'
#   Or symlink: ln -s /path/to/gemini-wrapper.sh /usr/local/bin/gemini-tracked

USAGE_FILE="${HOME}/.gemini/a2zusage-telemetry.jsonl"
REAL_GEMINI=$(which -a gemini | grep -v "gemini-wrapper" | head -1)

# If no real gemini found, try common paths
if [ -z "$REAL_GEMINI" ]; then
    for path in /opt/homebrew/bin/gemini /usr/local/bin/gemini ~/.local/bin/gemini; do
        if [ -x "$path" ]; then
            REAL_GEMINI="$path"
            break
        fi
    done
fi

if [ -z "$REAL_GEMINI" ]; then
    echo "Error: Could not find gemini CLI" >&2
    exit 1
fi

# Create usage file directory if needed
mkdir -p "$(dirname "$USAGE_FILE")"

# Check if user wants raw output (non-interactive mode)
if [[ "$*" == *"--output-format"* ]]; then
    # User specified format, run as-is
    exec "$REAL_GEMINI" "$@"
fi

# Run gemini with stream-json and process output
"$REAL_GEMINI" --output-format stream-json "$@" 2>&1 | while IFS= read -r line; do
    # Check if line is JSON
    if echo "$line" | grep -q '^{'; then
        # Parse the JSON event type
        event_type=$(echo "$line" | python3 -c "import sys,json; d=json.loads(sys.stdin.read()); print(d.get('type',''))" 2>/dev/null)
        
        case "$event_type" in
            "init")
                # Extract model info
                model=$(echo "$line" | python3 -c "import sys,json; d=json.loads(sys.stdin.read()); print(d.get('model','unknown'))" 2>/dev/null)
                session_id=$(echo "$line" | python3 -c "import sys,json; d=json.loads(sys.stdin.read()); print(d.get('session_id',''))" 2>/dev/null)
                ;;
            "message")
                # Display user/assistant messages
                role=$(echo "$line" | python3 -c "import sys,json; d=json.loads(sys.stdin.read()); print(d.get('role',''))" 2>/dev/null)
                content=$(echo "$line" | python3 -c "import sys,json; d=json.loads(sys.stdin.read()); print(d.get('content',''))" 2>/dev/null)
                if [ "$role" = "assistant" ]; then
                    echo -n "$content"
                fi
                ;;
            "result")
                # This is the usage data - log it!
                timestamp=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
                stats=$(echo "$line" | python3 -c "
import sys, json
d = json.loads(sys.stdin.read())
stats = d.get('stats', {})
print(json.dumps({
    'timestamp': '$timestamp',
    'model': '${model:-unknown}',
    'input_tokens': stats.get('input_tokens', stats.get('input', 0)),
    'output_tokens': stats.get('output_tokens', stats.get('output', 0)),
    'total_tokens': stats.get('total_tokens', 0),
    'cached_tokens': stats.get('cached', 0),
    'duration_ms': stats.get('duration_ms', 0),
    'tool_calls': stats.get('tool_calls', 0)
}))
" 2>/dev/null)
                
                if [ -n "$stats" ]; then
                    echo "$stats" >> "$USAGE_FILE"
                fi
                echo "" # Final newline after response
                ;;
        esac
    else
        # Non-JSON output (errors, loading messages, etc.)
        echo "$line" >&2
    fi
done
