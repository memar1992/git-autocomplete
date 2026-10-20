#Requires -Version 7.3
# Dot-source after posh-git. Other native completers can be passed explicitly.
param([scriptblock]$FallbackCompleter)

if (-not $FallbackCompleter -and (Get-Module posh-git)) {
    $FallbackCompleter = {
        param($wordToComplete, $commandAst, $cursorPosition)
        $length = $cursorPosition - $commandAst.Extent.StartOffset
        $line = $commandAst.ToString().PadRight($length, ' ').Substring(0, $length)
        posh-git\Expand-GitCommand $line
    }
}

$completer = {
    param($wordToComplete, $commandAst, $cursorPosition)
    $words = [System.Collections.Generic.List[string]]::new()
    $valid = $true
    $current = $false
    foreach ($element in $commandAst.CommandElements) {
        if ($element.Extent.StartOffset -gt $cursorPosition) { $valid = $false; break }
        $isString = $element -is [System.Management.Automation.Language.StringConstantExpressionAst]
        if ($element.Extent.EndOffset -ge $cursorPosition) {
            if ($isString -and $element.Value -eq '' -and
                $element.Extent.EndOffset -eq $commandAst.CommandElements[-1].Extent.EndOffset) {
                $words.Add('')
                $current = $true
            } else { $valid = $false }
            break
        }
        if ($isString) { $words.Add($element.Value) }
        elseif ($element -is [System.Management.Automation.Language.CommandParameterAst]) {
            $words.Add($element.Extent.Text)
        } else { $valid = $false; break }
    }
    if (-not $current) { $words.Add('') }
    $valid = $valid -and $words.Count -ge 4 -and $words[-1] -eq '' -and
        $words[-2] -in @('-m', '--message')
    if ($valid) {
        $executable = Get-Command git-autocomplete -CommandType Application -ErrorAction Ignore
    }
    if ($valid -and $executable) {
        # Standard argument passing preserves the final empty argument, including on Windows.
        $previousPassing = $PSNativeCommandArgumentPassing
        $nativeWords = $words.ToArray()
        $completed = $false
        try {
            $PSNativeCommandArgumentPassing = 'Standard'
            $program = $executable.Path
            $messages = @(& $program complete powershell -- @nativeWords 2>$null)
            $completed = $LASTEXITCODE -eq 0
        } catch {
            $completed = $false
        } finally {
            $PSNativeCommandArgumentPassing = $previousPassing
        }
        if ($completed) {
            foreach ($message in $messages) {
                $quoted = "'" + $message.Replace("'", "''") + "'"
                [System.Management.Automation.CompletionResult]::new($quoted, $message, 'ParameterValue', $message)
            }
            return
        }
    }
    if ($FallbackCompleter) { & $FallbackCompleter $wordToComplete $commandAst $cursorPosition }
}.GetNewClosure()
Register-ArgumentCompleter -Native -CommandName git,git.exe -ScriptBlock $completer
