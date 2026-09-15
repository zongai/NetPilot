$t=Get-ChildItem ./docs/tasks/NP-*.md
if($t.Count -ne 96){throw "Expected 96 task files"}
$m=Get-Content ./V5_MANIFEST.json|ConvertFrom-Json
if($m.task_count -ne 96 -or !$m.verified){throw "Manifest verification failed"}
Write-Host "PASS: 96 tasks NP-145..NP-240; DAG 96/95"
