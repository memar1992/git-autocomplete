#Requires -Version 7.3
$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
$temp = Join-Path ([IO.Path]::GetTempPath()) ('git-autocomplete-ps-' + [guid]::NewGuid())
$original = Get-Location
function Assert($condition, $message) { if (-not $condition) { throw $message } }
try {
    New-Item -ItemType Directory -Path $temp | Out-Null
    Set-Location $temp
    git init -q
    Assert ($LASTEXITCODE -eq 0) 'git init failed'
    'docs' | Set-Content README.md
    git add README.md
    Assert ($LASTEXITCODE -eq 0) 'git add failed'
    $fallback = {
        param($wordToComplete, $commandAst, $cursorPosition)
        if ($commandAst.ToString().StartsWith('git checkout')) {
            [System.Management.Automation.CompletionResult]::new('fixture', 'fixture', 'ParameterValue', 'fixture')
        }
    }
    . (Join-Path $root completions/git-autocomplete.ps1) -FallbackCompleter $fallback
    foreach ($line in @('git commit -m ', 'git commit --message ', 'git commit -m ""')) {
        $cursor = if ($line.EndsWith('""')) { $line.Length - 1 } else { $line.Length }
        $result = TabExpansion2 $line $cursor
        Assert ($result.CompletionMatches.Count -ge 1) "missing PowerShell suggestion: $line"
        Assert ($result.CompletionMatches[0].CompletionText -eq "'docs: add README'") 'incorrect quoting'
    }
    $line = 'git commit -m "my message"'
    $result = TabExpansion2 $line $line.Length
    Assert (-not ($result.CompletionMatches.ListItemText -contains 'docs: add README')) 'overwrote written message'
    $line = 'git commit -m  "my message"'
    $result = TabExpansion2 $line ($line.IndexOf('  ') + 1)
    Assert (-not ($result.CompletionMatches.ListItemText -contains 'docs: add README')) 'inserted ahead of written message'
    $line = 'git checkout fix'
    $result = TabExpansion2 $line $line.Length
    Assert ($result.CompletionMatches.ListItemText -contains 'fixture') 'fallback completion lost'
    # Use a legal Windows filename with PowerShell metacharacters and a single quote.
    git rm --cached -- README.md
    Assert ($LASTEXITCODE -eq 0) 'git rm failed'
    Remove-Item README.md
    $filename = 'Bob''s $(PWN);(x).md'
    'docs' | Set-Content -LiteralPath $filename
    git add --all
    $line = 'git commit -m '
    $result = TabExpansion2 $line $line.Length
    $completion = $result.CompletionMatches[0].CompletionText
    Assert ($completion -eq "'docs: add Bob''s `$(PWN);(x)'") "unsafe completion: $completion"
    $tokens = $null
    $errors = $null
    $ast = [System.Management.Automation.Language.Parser]::ParseInput("git commit -m $completion", [ref]$tokens, [ref]$errors)
    $elements = $ast.EndBlock.Statements[0].PipelineElements[0].CommandElements
    Assert ($errors.Count -eq 0 -and $elements.Count -eq 4) 'completion must parse as one argument'
    Assert (-not (Test-Path PWN)) 'shell injection occurred'
    Write-Output 'PowerShell native completion checks passed'
} finally {
    Set-Location $original
    Remove-Item -LiteralPath $temp -Recurse -Force
}
