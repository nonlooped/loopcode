# Minimal newline-delimited JSON-RPC MCP echo server for LoopCode tests (Windows).
$ErrorActionPreference = 'Stop'
while ($null -ne ($line = [Console]::In.ReadLine())) {
  $line = $line.Trim()
  if ([string]::IsNullOrWhiteSpace($line)) { continue }
  try {
    $msg = $line | ConvertFrom-Json
  } catch {
    continue
  }
  $method = $msg.method
  $mid = $msg.id
  if ($method -eq 'initialize') {
    $resp = @{
      jsonrpc = '2.0'
      id = $mid
      result = @{
        protocolVersion = '2024-11-05'
        capabilities = @{ tools = @{} }
        serverInfo = @{ name = 'loopcode-echo'; version = '1.0' }
      }
    } | ConvertTo-Json -Compress -Depth 6
    [Console]::Out.WriteLine($resp)
    [Console]::Out.Flush()
  }
  elseif ($method -eq 'notifications/initialized') {
    continue
  }
  elseif ($method -eq 'tools/call') {
    $paramsJson = ($msg.params | ConvertTo-Json -Compress -Depth 6)
    $resultObj = @{
      content = @(@{ type = 'text'; text = "echo:$paramsJson" })
      isError = $false
    }
    $resp = @{
      jsonrpc = '2.0'
      id = $mid
      result = $resultObj
    } | ConvertTo-Json -Compress -Depth 8
    [Console]::Out.WriteLine($resp)
    [Console]::Out.Flush()
    break
  }
  elseif ($null -ne $mid) {
    $resp = @{ jsonrpc = '2.0'; id = $mid; result = @{} } | ConvertTo-Json -Compress
    [Console]::Out.WriteLine($resp)
    [Console]::Out.Flush()
  }
}
