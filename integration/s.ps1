function Invoke-Shellmark {
    if (-Not (Get-Command shellmark -ErrorAction SilentlyContinue)) {
        Write-Output "shellmark not found in path"
        return
    }
    $OUT = shellmark --out powershell @args
    if ($OUT) {
        try {
            Invoke-Expression $OUT
        }
        catch {
            Write-Output $OUT
        }
    }
}

Set-Alias -Name {name} -Value Invoke-Shellmark