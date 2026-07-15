"""Minimal newline-delimited JSON-RPC MCP echo server for LoopCode tests."""
import sys
import json

def main():
    for line in sys.stdin:
        line = line.strip()
        if not line:
            continue
        try:
            msg = json.loads(line)
        except Exception:
            continue
        method = msg.get("method")
        mid = msg.get("id")
        if method == "initialize":
            resp = {
                "jsonrpc": "2.0",
                "id": mid,
                "result": {
                    "protocolVersion": "2024-11-05",
                    "capabilities": {"tools": {}},
                    "serverInfo": {"name": "loopcode-echo", "version": "1.0"},
                },
            }
            sys.stdout.write(json.dumps(resp) + "\n")
            sys.stdout.flush()
        elif method == "notifications/initialized":
            continue
        elif method == "tools/call":
            params = msg.get("params") or {}
            result = {
                "content": [
                    {
                        "type": "text",
                        "text": "echo:" + json.dumps(params),
                    }
                ],
                "isError": False,
            }
            resp = {"jsonrpc": "2.0", "id": mid, "result": result}
            sys.stdout.write(json.dumps(resp) + "\n")
            sys.stdout.flush()
            break
        elif mid is not None:
            resp = {"jsonrpc": "2.0", "id": mid, "result": {}}
            sys.stdout.write(json.dumps(resp) + "\n")
            sys.stdout.flush()

if __name__ == "__main__":
    main()
