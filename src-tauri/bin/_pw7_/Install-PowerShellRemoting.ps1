# Copyright (c) Microsoft Corporation.
# Licensed under the MIT License.

#####################################################################################################
#
# Registers the WinRM endpoint for this instance of PowerShell.
#
# If the parameters '-PowerShellHome' were specified, it means that the script will be registering
# an instance of PowerShell from another instance of PowerShell.
#
# If no parameter is specified, it means that this instance of PowerShell is registering itself.
#
# Assumptions:
#     1. The CoreCLR and the PowerShell assemblies are side-by-side in $PSHOME
#     2. Plugins are registered by version number. Only one plugin can be automatically registered
#        per PowerShell version. However, multiple endpoints may be manually registered for a given
#        plugin.
#
#####################################################################################################
[CmdletBinding(DefaultParameterSetName = "NotByPath")]
param
(
    [parameter(Mandatory = $true, ParameterSetName = "ByPath")]
    [switch]$Force,
    [string]
    $PowerShellHome
)

Set-StrictMode -Version 3.0

if (! ([Security.Principal.WindowsPrincipal] [Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole] "Administrator"))
{
    Write-Error "WinRM registration requires Administrator rights. To run this cmdlet, start PowerShell with the `"Run as administrator`" option."
    return
}
function Register-WinRmPlugin
{
    param
    (
        #
        # Expected Example:
        # %windir%\\system32\\PowerShell\\6.0.0\\pwrshplugin.dll
        #
        [string]
        [parameter(Mandatory=$true)]
        [ValidateNotNullOrEmpty()]
        $pluginAbsolutePath,

        #
        # Expected Example: powershell.6.0.0-beta.3
        #
        [string]
        [parameter(Mandatory=$true)]
        [ValidateNotNullOrEmpty()]
        $pluginEndpointName
    )

    $regKey = "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\WSMAN\Plugin\$pluginEndpointName"

    $pluginArchitecture = "64"
    if ($env:PROCESSOR_ARCHITECTURE -match "x86" -or $env:PROCESSOR_ARCHITECTURE -eq "ARM")
    {
        $pluginArchitecture = "32"
    }
    $regKeyValueFormatString = @"
<PlugInConfiguration xmlns="http://schemas.microsoft.com/wbem/wsman/1/config/PluginConfiguration" Name="{0}" Filename="{1}"
    SDKVersion="2" XmlRenderingType="text" Enabled="True" OutputBufferingMode="Block" ProcessIdleTimeoutSec="0" Architecture="{2}"
    UseSharedProcess="false" RunAsUser="" RunAsPassword="" AutoRestart="false">
    <InitializationParameters>
        <Param Name="PSVersion" Value="7.0"/>
    </InitializationParameters>
    <Resources>
        <Resource ResourceUri="http://schemas.microsoft.com/powershell/{0}" SupportsOptions="true" ExactMatch="true">
            <Security Uri="http://schemas.microsoft.com/powershell/{0}" ExactMatch="true"
            Sddl="O:NSG:BAD:P(A;;GA;;;BA)S:P(AU;FA;GA;;;WD)(AU;SA;GXGW;;;WD)"/>
            <Capability Type="Shell"/>
        </Resource>
    </Resources>
    <Quotas IdleTimeoutms="7200000" MaxConcurrentUsers="5" MaxProcessesPerShell="15" MaxMemoryPerShellMB="1024" MaxShellsPerUser="25"
    MaxConcurrentCommandsPerShell="1000" MaxShells="25" MaxIdleTimeoutms="43200000"/>
</PlugInConfiguration>
"@
    $valueString = $regKeyValueFormatString -f $pluginEndpointName, $pluginAbsolutePath, $pluginArchitecture

    New-Item $regKey -Force > $null
    New-ItemProperty -Path $regKey -Name ConfigXML -Value $valueString > $null
}

function New-PluginConfigFile
{
    [CmdletBinding(SupportsShouldProcess, ConfirmImpact="Medium")]
    param
    (
        [string]
        [parameter(Mandatory=$true)]
        [ValidateNotNullOrEmpty()]
        $pluginFile,

        [string]
        [parameter(Mandatory=$true)]
        [ValidateNotNullOrEmpty()]
        $targetPsHomeDir
    )

    # This always overwrites the file with a new version of it if the
    # script is invoked multiple times.
    Set-Content -Path $pluginFile -Value "PSHOMEDIR=$targetPsHomeDir" -ErrorAction Stop
    Add-Content -Path $pluginFile -Value "CORECLRDIR=$targetPsHomeDir" -ErrorAction Stop

    Write-Verbose "Created Plugin Config File: $pluginFile" -Verbose
}

function Install-PluginEndpoint {
    [CmdletBinding(SupportsShouldProcess, ConfirmImpact="Medium")]
    param (
        [Parameter()] [bool] $Force,
        [switch]
        $VersionIndependent
    )

    ######################
    #                    #
    # Install the plugin #
    #                    #
    ######################

    if (-not [String]::IsNullOrEmpty($PowerShellHome))
    {
        $targetPsHome = $PowerShellHome
        $targetPsVersion = & "$targetPsHome\pwsh" -NoProfile -Command '$PSVersionTable.PSVersion.ToString()'
    }
    else
    {
        ## Get the PSHome and PSVersion using the current powershell instance
        $targetPsHome = $PSHOME
        $targetPsVersion = $PSVersionTable.PSVersion.ToString()
    }
    Write-Verbose "PowerShellHome: $targetPsHome" -Verbose

    # For default, not tied to the specific version endpoint, we apply
    # only first number in the PSVersion string to the endpoint name.
    # Example name: 'PowerShell.6'.
    if ($VersionIndependent) {
        $dotPos = $targetPsVersion.IndexOf(".")
        if ($dotPos -ne -1) {
            $targetPsVersion = $targetPsVersion.Substring(0, $dotPos)
        }
    }

    Write-Verbose "Using PowerShell Version: $targetPsVersion" -Verbose

    $pluginEndpointName = "PowerShell.$targetPsVersion"

    $endPoint = Get-PSSessionConfiguration $pluginEndpointName -Force:$Force -ErrorAction silentlycontinue 2>&1

    # If endpoint exists and -Force parameter was not used, the endpoint would not be overwritten.
    if ($endpoint -and !$Force)
    {
        Write-Error -Category ResourceExists -ErrorId "PSSessionConfigurationExists" -Message "Endpoint $pluginEndpointName already exists."
        return
    }

    if (!$PSCmdlet.ShouldProcess($pluginEndpointName)) {
        return
    }

    if ($PSVersionTable.PSVersion -lt "6.0")
    {
        # This script is primarily used from Windows PowerShell for Win10 IoT and NanoServer to setup PSCore6 remoting endpoint
        # so it's ok to hardcode to 'C:\Windows' for those systems
        $pluginBasePath = Join-Path "C:\Windows\System32\PowerShell" $targetPsVersion
    }
    else
    {
        $pluginBasePath = Join-Path ([System.Environment]::GetFolderPath([System.Environment+SpecialFolder]::Windows) + "\System32\PowerShell") $targetPsVersion
    }

    $resolvedPluginAbsolutePath = ""
    if (! (Test-Path $pluginBasePath))
    {
        Write-Verbose "Creating $pluginBasePath"
        $resolvedPluginAbsolutePath = New-Item -Type Directory -Path $pluginBasePath
    }
    else
    {
        $resolvedPluginAbsolutePath = Resolve-Path $pluginBasePath
    }

    $pluginPath = Join-Path $resolvedPluginAbsolutePath "pwrshplugin.dll"

    # This is forced to ensure the file is placed correctly
    Copy-Item $targetPsHome\pwrshplugin.dll $resolvedPluginAbsolutePath -Force -Verbose -ErrorAction Stop

    $pluginFile = Join-Path $resolvedPluginAbsolutePath "RemotePowerShellConfig.txt"
    New-PluginConfigFile $pluginFile (Resolve-Path $targetPsHome)

    # Register the plugin
    Register-WinRmPlugin $pluginPath $pluginEndpointName

    ####################################################################
    #                                                                  #
    # Validations to confirm that everything was registered correctly. #
    #                                                                  #
    ####################################################################

    if (! (Test-Path $pluginFile))
    {
        throw "WinRM Plugin configuration file not created. Expected = $pluginFile"
    }

    if (! (Test-Path $resolvedPluginAbsolutePath\pwrshplugin.dll))
    {
        throw "WinRM Plugin DLL missing. Expected = $resolvedPluginAbsolutePath\pwrshplugin.dll"
    }

    try
    {
        Write-Host "`nGet-PSSessionConfiguration $pluginEndpointName" -ForegroundColor "green"
        Get-PSSessionConfiguration $pluginEndpointName -ErrorAction Stop
    }
    catch [Microsoft.PowerShell.Commands.WriteErrorException]
    {
        throw "No remoting session configuration matches the name $pluginEndpointName."
    }
}

Install-PluginEndpoint -Force $Force
Install-PluginEndpoint -Force $Force -VersionIndependent

Write-Host "Restarting WinRM to ensure that the plugin configuration change takes effect.`nThis is required for WinRM running on Windows SKUs prior to Windows 10." -ForegroundColor Magenta
Restart-Service winrm


# SIG # Begin signature block
# MIIoKQYJKoZIhvcNAQcCoIIoGjCCKBYCAQExDzANBglghkgBZQMEAgEFADB5Bgor
# BgEEAYI3AgEEoGswaTA0BgorBgEEAYI3AgEeMCYCAwEAAAQQH8w7YFlLCE63JNLG
# KX7zUQIBAAIBAAIBAAIBAAIBADAxMA0GCWCGSAFlAwQCAQUABCAmMBO/z6uvMDNL
# ywsEntW1l8ui09V7mB9QxyD4sH3/L6CCDXYwggX0MIID3KADAgECAhMzAAADrzBA
# DkyjTQVBAAAAAAOvMA0GCSqGSIb3DQEBCwUAMH4xCzAJBgNVBAYTAlVTMRMwEQYD
# VQQIEwpXYXNoaW5ndG9uMRAwDgYDVQQHEwdSZWRtb25kMR4wHAYDVQQKExVNaWNy
# b3NvZnQgQ29ycG9yYXRpb24xKDAmBgNVBAMTH01pY3Jvc29mdCBDb2RlIFNpZ25p
# bmcgUENBIDIwMTEwHhcNMjMxMTE2MTkwOTAwWhcNMjQxMTE0MTkwOTAwWjB0MQsw
# CQYDVQQGEwJVUzETMBEGA1UECBMKV2FzaGluZ3RvbjEQMA4GA1UEBxMHUmVkbW9u
# ZDEeMBwGA1UEChMVTWljcm9zb2Z0IENvcnBvcmF0aW9uMR4wHAYDVQQDExVNaWNy
# b3NvZnQgQ29ycG9yYXRpb24wggEiMA0GCSqGSIb3DQEBAQUAA4IBDwAwggEKAoIB
# AQDOS8s1ra6f0YGtg0OhEaQa/t3Q+q1MEHhWJhqQVuO5amYXQpy8MDPNoJYk+FWA
# hePP5LxwcSge5aen+f5Q6WNPd6EDxGzotvVpNi5ve0H97S3F7C/axDfKxyNh21MG
# 0W8Sb0vxi/vorcLHOL9i+t2D6yvvDzLlEefUCbQV/zGCBjXGlYJcUj6RAzXyeNAN
# xSpKXAGd7Fh+ocGHPPphcD9LQTOJgG7Y7aYztHqBLJiQQ4eAgZNU4ac6+8LnEGAL
# go1ydC5BJEuJQjYKbNTy959HrKSu7LO3Ws0w8jw6pYdC1IMpdTkk2puTgY2PDNzB
# tLM4evG7FYer3WX+8t1UMYNTAgMBAAGjggFzMIIBbzAfBgNVHSUEGDAWBgorBgEE
# AYI3TAgBBggrBgEFBQcDAzAdBgNVHQ4EFgQURxxxNPIEPGSO8kqz+bgCAQWGXsEw
# RQYDVR0RBD4wPKQ6MDgxHjAcBgNVBAsTFU1pY3Jvc29mdCBDb3Jwb3JhdGlvbjEW
# MBQGA1UEBRMNMjMwMDEyKzUwMTgyNjAfBgNVHSMEGDAWgBRIbmTlUAXTgqoXNzci
# tW2oynUClTBUBgNVHR8ETTBLMEmgR6BFhkNodHRwOi8vd3d3Lm1pY3Jvc29mdC5j
# b20vcGtpb3BzL2NybC9NaWNDb2RTaWdQQ0EyMDExXzIwMTEtMDctMDguY3JsMGEG
# CCsGAQUFBwEBBFUwUzBRBggrBgEFBQcwAoZFaHR0cDovL3d3dy5taWNyb3NvZnQu
# Y29tL3BraW9wcy9jZXJ0cy9NaWNDb2RTaWdQQ0EyMDExXzIwMTEtMDctMDguY3J0
# MAwGA1UdEwEB/wQCMAAwDQYJKoZIhvcNAQELBQADggIBAISxFt/zR2frTFPB45Yd
# mhZpB2nNJoOoi+qlgcTlnO4QwlYN1w/vYwbDy/oFJolD5r6FMJd0RGcgEM8q9TgQ
# 2OC7gQEmhweVJ7yuKJlQBH7P7Pg5RiqgV3cSonJ+OM4kFHbP3gPLiyzssSQdRuPY
# 1mIWoGg9i7Y4ZC8ST7WhpSyc0pns2XsUe1XsIjaUcGu7zd7gg97eCUiLRdVklPmp
# XobH9CEAWakRUGNICYN2AgjhRTC4j3KJfqMkU04R6Toyh4/Toswm1uoDcGr5laYn
# TfcX3u5WnJqJLhuPe8Uj9kGAOcyo0O1mNwDa+LhFEzB6CB32+wfJMumfr6degvLT
# e8x55urQLeTjimBQgS49BSUkhFN7ois3cZyNpnrMca5AZaC7pLI72vuqSsSlLalG
# OcZmPHZGYJqZ0BacN274OZ80Q8B11iNokns9Od348bMb5Z4fihxaBWebl8kWEi2O
# PvQImOAeq3nt7UWJBzJYLAGEpfasaA3ZQgIcEXdD+uwo6ymMzDY6UamFOfYqYWXk
# ntxDGu7ngD2ugKUuccYKJJRiiz+LAUcj90BVcSHRLQop9N8zoALr/1sJuwPrVAtx
# HNEgSW+AKBqIxYWM4Ev32l6agSUAezLMbq5f3d8x9qzT031jMDT+sUAoCw0M5wVt
# CUQcqINPuYjbS1WgJyZIiEkBMIIHejCCBWKgAwIBAgIKYQ6Q0gAAAAAAAzANBgkq
# hkiG9w0BAQsFADCBiDELMAkGA1UEBhMCVVMxEzARBgNVBAgTCldhc2hpbmd0b24x
# EDAOBgNVBAcTB1JlZG1vbmQxHjAcBgNVBAoTFU1pY3Jvc29mdCBDb3Jwb3JhdGlv
# bjEyMDAGA1UEAxMpTWljcm9zb2Z0IFJvb3QgQ2VydGlmaWNhdGUgQXV0aG9yaXR5
# IDIwMTEwHhcNMTEwNzA4MjA1OTA5WhcNMjYwNzA4MjEwOTA5WjB+MQswCQYDVQQG
# EwJVUzETMBEGA1UECBMKV2FzaGluZ3RvbjEQMA4GA1UEBxMHUmVkbW9uZDEeMBwG
# A1UEChMVTWljcm9zb2Z0IENvcnBvcmF0aW9uMSgwJgYDVQQDEx9NaWNyb3NvZnQg
# Q29kZSBTaWduaW5nIFBDQSAyMDExMIICIjANBgkqhkiG9w0BAQEFAAOCAg8AMIIC
# CgKCAgEAq/D6chAcLq3YbqqCEE00uvK2WCGfQhsqa+laUKq4BjgaBEm6f8MMHt03
# a8YS2AvwOMKZBrDIOdUBFDFC04kNeWSHfpRgJGyvnkmc6Whe0t+bU7IKLMOv2akr
# rnoJr9eWWcpgGgXpZnboMlImEi/nqwhQz7NEt13YxC4Ddato88tt8zpcoRb0Rrrg
# OGSsbmQ1eKagYw8t00CT+OPeBw3VXHmlSSnnDb6gE3e+lD3v++MrWhAfTVYoonpy
# 4BI6t0le2O3tQ5GD2Xuye4Yb2T6xjF3oiU+EGvKhL1nkkDstrjNYxbc+/jLTswM9
# sbKvkjh+0p2ALPVOVpEhNSXDOW5kf1O6nA+tGSOEy/S6A4aN91/w0FK/jJSHvMAh
# dCVfGCi2zCcoOCWYOUo2z3yxkq4cI6epZuxhH2rhKEmdX4jiJV3TIUs+UsS1Vz8k
# A/DRelsv1SPjcF0PUUZ3s/gA4bysAoJf28AVs70b1FVL5zmhD+kjSbwYuER8ReTB
# w3J64HLnJN+/RpnF78IcV9uDjexNSTCnq47f7Fufr/zdsGbiwZeBe+3W7UvnSSmn
# Eyimp31ngOaKYnhfsi+E11ecXL93KCjx7W3DKI8sj0A3T8HhhUSJxAlMxdSlQy90
# lfdu+HggWCwTXWCVmj5PM4TasIgX3p5O9JawvEagbJjS4NaIjAsCAwEAAaOCAe0w
# ggHpMBAGCSsGAQQBgjcVAQQDAgEAMB0GA1UdDgQWBBRIbmTlUAXTgqoXNzcitW2o
# ynUClTAZBgkrBgEEAYI3FAIEDB4KAFMAdQBiAEMAQTALBgNVHQ8EBAMCAYYwDwYD
# VR0TAQH/BAUwAwEB/zAfBgNVHSMEGDAWgBRyLToCMZBDuRQFTuHqp8cx0SOJNDBa
# BgNVHR8EUzBRME+gTaBLhklodHRwOi8vY3JsLm1pY3Jvc29mdC5jb20vcGtpL2Ny
# bC9wcm9kdWN0cy9NaWNSb29DZXJBdXQyMDExXzIwMTFfMDNfMjIuY3JsMF4GCCsG
# AQUFBwEBBFIwUDBOBggrBgEFBQcwAoZCaHR0cDovL3d3dy5taWNyb3NvZnQuY29t
# L3BraS9jZXJ0cy9NaWNSb29DZXJBdXQyMDExXzIwMTFfMDNfMjIuY3J0MIGfBgNV
# HSAEgZcwgZQwgZEGCSsGAQQBgjcuAzCBgzA/BggrBgEFBQcCARYzaHR0cDovL3d3
# dy5taWNyb3NvZnQuY29tL3BraW9wcy9kb2NzL3ByaW1hcnljcHMuaHRtMEAGCCsG
# AQUFBwICMDQeMiAdAEwAZQBnAGEAbABfAHAAbwBsAGkAYwB5AF8AcwB0AGEAdABl
# AG0AZQBuAHQALiAdMA0GCSqGSIb3DQEBCwUAA4ICAQBn8oalmOBUeRou09h0ZyKb
# C5YR4WOSmUKWfdJ5DJDBZV8uLD74w3LRbYP+vj/oCso7v0epo/Np22O/IjWll11l
# hJB9i0ZQVdgMknzSGksc8zxCi1LQsP1r4z4HLimb5j0bpdS1HXeUOeLpZMlEPXh6
# I/MTfaaQdION9MsmAkYqwooQu6SpBQyb7Wj6aC6VoCo/KmtYSWMfCWluWpiW5IP0
# wI/zRive/DvQvTXvbiWu5a8n7dDd8w6vmSiXmE0OPQvyCInWH8MyGOLwxS3OW560
# STkKxgrCxq2u5bLZ2xWIUUVYODJxJxp/sfQn+N4sOiBpmLJZiWhub6e3dMNABQam
# ASooPoI/E01mC8CzTfXhj38cbxV9Rad25UAqZaPDXVJihsMdYzaXht/a8/jyFqGa
# J+HNpZfQ7l1jQeNbB5yHPgZ3BtEGsXUfFL5hYbXw3MYbBL7fQccOKO7eZS/sl/ah
# XJbYANahRr1Z85elCUtIEJmAH9AAKcWxm6U/RXceNcbSoqKfenoi+kiVH6v7RyOA
# 9Z74v2u3S5fi63V4GuzqN5l5GEv/1rMjaHXmr/r8i+sLgOppO6/8MO0ETI7f33Vt
# Y5E90Z1WTk+/gFcioXgRMiF670EKsT/7qMykXcGhiJtXcVZOSEXAQsmbdlsKgEhr
# /Xmfwb1tbWrJUnMTDXpQzTGCGgkwghoFAgEBMIGVMH4xCzAJBgNVBAYTAlVTMRMw
# EQYDVQQIEwpXYXNoaW5ndG9uMRAwDgYDVQQHEwdSZWRtb25kMR4wHAYDVQQKExVN
# aWNyb3NvZnQgQ29ycG9yYXRpb24xKDAmBgNVBAMTH01pY3Jvc29mdCBDb2RlIFNp
# Z25pbmcgUENBIDIwMTECEzMAAAOvMEAOTKNNBUEAAAAAA68wDQYJYIZIAWUDBAIB
# BQCgga4wGQYJKoZIhvcNAQkDMQwGCisGAQQBgjcCAQQwHAYKKwYBBAGCNwIBCzEO
# MAwGCisGAQQBgjcCARUwLwYJKoZIhvcNAQkEMSIEIASzAbkapgzkIHyjEtglU6vA
# HP0o6aqvz8pdcZCJCVXCMEIGCisGAQQBgjcCAQwxNDAyoBSAEgBNAGkAYwByAG8A
# cwBvAGYAdKEagBhodHRwOi8vd3d3Lm1pY3Jvc29mdC5jb20wDQYJKoZIhvcNAQEB
# BQAEggEAUojy/6UU4L3Vrn/dK+XJByO4hTJOqIPTzWVdj5ai/pNX+eUNphh8Tf0a
# xHN7BsJqb2Qgq8YNMrEB6Sxl61PqD1LEURtyKJYjk8b8K7Kfrhi64VAYHmi0OWjq
# aJx+EfcvDB7dP5FT9PH/Z5uv4SAiByOv5z1X0YvJBNIFDmTtlvMIqQjRtrJvZO9a
# 7o8iJGnxo5Vndllw9S+ZIIMkncs4UQAr0wui06M6LlRp91xmUwEPd/xe5uorliBv
# A6ztNcGc/IiBjoT1hmppbilf4cNI3Wvg/Q8lc91kVRl0l0eqMMmn2jFxXe9NgALo
# Fbvo8me3db60yjPg0t51gWVQnq2naKGCF5MwghePBgorBgEEAYI3AwMBMYIXfzCC
# F3sGCSqGSIb3DQEHAqCCF2wwghdoAgEDMQ8wDQYJYIZIAWUDBAIBBQAwggFRBgsq
# hkiG9w0BCRABBKCCAUAEggE8MIIBOAIBAQYKKwYBBAGEWQoDATAxMA0GCWCGSAFl
# AwQCAQUABCA665q8c4j/x8zJ68EYw6yuGbLJV5LA3xdRxe9zMCIbFAIGZpVdr/M5
# GBIyMDI0MDcxNzAxMTQ1Ny45NlowBIACAfSggdGkgc4wgcsxCzAJBgNVBAYTAlVT
# MRMwEQYDVQQIEwpXYXNoaW5ndG9uMRAwDgYDVQQHEwdSZWRtb25kMR4wHAYDVQQK
# ExVNaWNyb3NvZnQgQ29ycG9yYXRpb24xJTAjBgNVBAsTHE1pY3Jvc29mdCBBbWVy
# aWNhIE9wZXJhdGlvbnMxJzAlBgNVBAsTHm5TaGllbGQgVFNTIEVTTjo5MjAwLTA1
# RTAtRDk0NzElMCMGA1UEAxMcTWljcm9zb2Z0IFRpbWUtU3RhbXAgU2VydmljZaCC
# EeowggcgMIIFCKADAgECAhMzAAAB5y6PL5MLTxvpAAEAAAHnMA0GCSqGSIb3DQEB
# CwUAMHwxCzAJBgNVBAYTAlVTMRMwEQYDVQQIEwpXYXNoaW5ndG9uMRAwDgYDVQQH
# EwdSZWRtb25kMR4wHAYDVQQKExVNaWNyb3NvZnQgQ29ycG9yYXRpb24xJjAkBgNV
# BAMTHU1pY3Jvc29mdCBUaW1lLVN0YW1wIFBDQSAyMDEwMB4XDTIzMTIwNjE4NDUx
# OVoXDTI1MDMwNTE4NDUxOVowgcsxCzAJBgNVBAYTAlVTMRMwEQYDVQQIEwpXYXNo
# aW5ndG9uMRAwDgYDVQQHEwdSZWRtb25kMR4wHAYDVQQKExVNaWNyb3NvZnQgQ29y
# cG9yYXRpb24xJTAjBgNVBAsTHE1pY3Jvc29mdCBBbWVyaWNhIE9wZXJhdGlvbnMx
# JzAlBgNVBAsTHm5TaGllbGQgVFNTIEVTTjo5MjAwLTA1RTAtRDk0NzElMCMGA1UE
# AxMcTWljcm9zb2Z0IFRpbWUtU3RhbXAgU2VydmljZTCCAiIwDQYJKoZIhvcNAQEB
# BQADggIPADCCAgoCggIBAMJXny/gi5Drn1c8zUO1pYy/38dFQLmR2IQXz1gE/r9G
# fuSOoyRnkRJ6Z/kSWLgIu1BVJ59GkXWPtLkssqKwxY4ZFotxpVsZN9yYjW8xEnW3
# MzAI0igKr+/LxYfxB1XUH8Bvmwr5D3Ii/MbDjtN9c8TxGWtq7Ar976dafAy3TrRq
# QRmIknPVWHUuFJgpqI/1nbcRmYYRMJaKCQpty4CeG+HfKsxrz24F9p4dBkQcZCp2
# yQzjwQFxZJZ2mJJIGIDHKEdSRuSeX08/O0H9JTHNFmNTNYeD1t/WapnRwiIBYLQS
# Mrs42GVB8pJEdUsos0+mXf/5QvheNzRi92pzzyA4tSv/zhP3/Ermvza6W9GnYDz9
# qv1wbhbvrnS4poDFECaAviEqAhfn/RogCxvKok5ro4gZIX1r4N9eXUulA80pHv3a
# xwXu2MPlarAi6J9L1hSIcy9EuOMqTRJIJX+alcLQGg+STlqx/GuslsKwl48dI4Ru
# WknNGbNo/o4xfBFytvtNcVA6xOQq6qRa+9gg+9XMLrxQz4yyQs+V3V6p044wrtJt
# t/a0ZJl/f6I7BZAxxZcH2DDmArcAhgrTxaQkm7LM+p+K2C5t1EKZiv0JWw065b7A
# cNgaFyIkMXYuSuOQVSNRxdIgl31/ayxiK1n0K6sZXvgFBx+vGO+TUvyO+03ua6Uj
# AgMBAAGjggFJMIIBRTAdBgNVHQ4EFgQUz/7gmICfNjh2kR/9mWuHUrvej1gwHwYD
# VR0jBBgwFoAUn6cVXQBeYl2D9OXSZacbUzUZ6XIwXwYDVR0fBFgwVjBUoFKgUIZO
# aHR0cDovL3d3dy5taWNyb3NvZnQuY29tL3BraW9wcy9jcmwvTWljcm9zb2Z0JTIw
# VGltZS1TdGFtcCUyMFBDQSUyMDIwMTAoMSkuY3JsMGwGCCsGAQUFBwEBBGAwXjBc
# BggrBgEFBQcwAoZQaHR0cDovL3d3dy5taWNyb3NvZnQuY29tL3BraW9wcy9jZXJ0
# cy9NaWNyb3NvZnQlMjBUaW1lLVN0YW1wJTIwUENBJTIwMjAxMCgxKS5jcnQwDAYD
# VR0TAQH/BAIwADAWBgNVHSUBAf8EDDAKBggrBgEFBQcDCDAOBgNVHQ8BAf8EBAMC
# B4AwDQYJKoZIhvcNAQELBQADggIBAHSh8NuT6WVaLVwLqex+J7km2nT2jpvoBEKm
# +0M+rYoU/6GL5Q00/ssZyIq5ySpcKYFMUiF8F4ZLG+TrJyiR1CvfzXmkQ5phZOce
# 9DT7yErLzqvUXit8G7igcHlxPLTxPiiGsb85gb8H+A2fPQ6Xq/u7+oSPPjzNdnpm
# XEobJnAqYplZoF3YNgTDMql0uQHGzoDp6dZlHSNj6rkV1tXjmCEZMqBKvkQIA6cs
# PieMnB+MirSZFlbANlChe0lJpUdK7aUdAvdgcQWKS6dtRMl818EMsvsa/6xOZGIN
# mTLk4DGgsbaBpN+6IVt+mZJ89yCXkI5TN8xCfOkp9fr4WQjRBA2+4+lawNTyxH66
# eLZWYOjuuaomuibiKGBU10tox81Sq8EvlmJIrXOZoQsEn1r5g6MTmmZJqtbmwZuf
# uJWQXZb0lAg4fq0ZYsUlLkezfrNqGSgeHyIP3rct4aNmqQW6wppRbvbIyP/LFN4Y
# QM6givfmTBfGvVS77OS6vbL4W41jShmOmnOn3kBbWV6E/TFo76gFXVd+9oK6v8Hk
# 9UCnbHOuiwwRRwDCkmmKj5Vh8i58aPuZ5dwZBhYDxSavwroC6j4mWPwh4VLqVK8q
# GpCmZ0HMAwao85Aq3U7DdlfF6Eru8CKKbdmIAuUzQrnjqTSxmvF1k+CmbPs7zD2A
# cu7JkBB7MIIHcTCCBVmgAwIBAgITMwAAABXF52ueAptJmQAAAAAAFTANBgkqhkiG
# 9w0BAQsFADCBiDELMAkGA1UEBhMCVVMxEzARBgNVBAgTCldhc2hpbmd0b24xEDAO
# BgNVBAcTB1JlZG1vbmQxHjAcBgNVBAoTFU1pY3Jvc29mdCBDb3Jwb3JhdGlvbjEy
# MDAGA1UEAxMpTWljcm9zb2Z0IFJvb3QgQ2VydGlmaWNhdGUgQXV0aG9yaXR5IDIw
# MTAwHhcNMjEwOTMwMTgyMjI1WhcNMzAwOTMwMTgzMjI1WjB8MQswCQYDVQQGEwJV
# UzETMBEGA1UECBMKV2FzaGluZ3RvbjEQMA4GA1UEBxMHUmVkbW9uZDEeMBwGA1UE
# ChMVTWljcm9zb2Z0IENvcnBvcmF0aW9uMSYwJAYDVQQDEx1NaWNyb3NvZnQgVGlt
# ZS1TdGFtcCBQQ0EgMjAxMDCCAiIwDQYJKoZIhvcNAQEBBQADggIPADCCAgoCggIB
# AOThpkzntHIhC3miy9ckeb0O1YLT/e6cBwfSqWxOdcjKNVf2AX9sSuDivbk+F2Az
# /1xPx2b3lVNxWuJ+Slr+uDZnhUYjDLWNE893MsAQGOhgfWpSg0S3po5GawcU88V2
# 9YZQ3MFEyHFcUTE3oAo4bo3t1w/YJlN8OWECesSq/XJprx2rrPY2vjUmZNqYO7oa
# ezOtgFt+jBAcnVL+tuhiJdxqD89d9P6OU8/W7IVWTe/dvI2k45GPsjksUZzpcGkN
# yjYtcI4xyDUoveO0hyTD4MmPfrVUj9z6BVWYbWg7mka97aSueik3rMvrg0XnRm7K
# MtXAhjBcTyziYrLNueKNiOSWrAFKu75xqRdbZ2De+JKRHh09/SDPc31BmkZ1zcRf
# NN0Sidb9pSB9fvzZnkXftnIv231fgLrbqn427DZM9ituqBJR6L8FA6PRc6ZNN3SU
# HDSCD/AQ8rdHGO2n6Jl8P0zbr17C89XYcz1DTsEzOUyOArxCaC4Q6oRRRuLRvWoY
# WmEBc8pnol7XKHYC4jMYctenIPDC+hIK12NvDMk2ZItboKaDIV1fMHSRlJTYuVD5
# C4lh8zYGNRiER9vcG9H9stQcxWv2XFJRXRLbJbqvUAV6bMURHXLvjflSxIUXk8A8
# FdsaN8cIFRg/eKtFtvUeh17aj54WcmnGrnu3tz5q4i6tAgMBAAGjggHdMIIB2TAS
# BgkrBgEEAYI3FQEEBQIDAQABMCMGCSsGAQQBgjcVAgQWBBQqp1L+ZMSavoKRPEY1
# Kc8Q/y8E7jAdBgNVHQ4EFgQUn6cVXQBeYl2D9OXSZacbUzUZ6XIwXAYDVR0gBFUw
# UzBRBgwrBgEEAYI3TIN9AQEwQTA/BggrBgEFBQcCARYzaHR0cDovL3d3dy5taWNy
# b3NvZnQuY29tL3BraW9wcy9Eb2NzL1JlcG9zaXRvcnkuaHRtMBMGA1UdJQQMMAoG
# CCsGAQUFBwMIMBkGCSsGAQQBgjcUAgQMHgoAUwB1AGIAQwBBMAsGA1UdDwQEAwIB
# hjAPBgNVHRMBAf8EBTADAQH/MB8GA1UdIwQYMBaAFNX2VsuP6KJcYmjRPZSQW9fO
# mhjEMFYGA1UdHwRPME0wS6BJoEeGRWh0dHA6Ly9jcmwubWljcm9zb2Z0LmNvbS9w
# a2kvY3JsL3Byb2R1Y3RzL01pY1Jvb0NlckF1dF8yMDEwLTA2LTIzLmNybDBaBggr
# BgEFBQcBAQROMEwwSgYIKwYBBQUHMAKGPmh0dHA6Ly93d3cubWljcm9zb2Z0LmNv
# bS9wa2kvY2VydHMvTWljUm9vQ2VyQXV0XzIwMTAtMDYtMjMuY3J0MA0GCSqGSIb3
# DQEBCwUAA4ICAQCdVX38Kq3hLB9nATEkW+Geckv8qW/qXBS2Pk5HZHixBpOXPTEz
# tTnXwnE2P9pkbHzQdTltuw8x5MKP+2zRoZQYIu7pZmc6U03dmLq2HnjYNi6cqYJW
# AAOwBb6J6Gngugnue99qb74py27YP0h1AdkY3m2CDPVtI1TkeFN1JFe53Z/zjj3G
# 82jfZfakVqr3lbYoVSfQJL1AoL8ZthISEV09J+BAljis9/kpicO8F7BUhUKz/Aye
# ixmJ5/ALaoHCgRlCGVJ1ijbCHcNhcy4sa3tuPywJeBTpkbKpW99Jo3QMvOyRgNI9
# 5ko+ZjtPu4b6MhrZlvSP9pEB9s7GdP32THJvEKt1MMU0sHrYUP4KWN1APMdUbZ1j
# dEgssU5HLcEUBHG/ZPkkvnNtyo4JvbMBV0lUZNlz138eW0QBjloZkWsNn6Qo3GcZ
# KCS6OEuabvshVGtqRRFHqfG3rsjoiV5PndLQTHa1V1QJsWkBRH58oWFsc/4Ku+xB
# Zj1p/cvBQUl+fpO+y/g75LcVv7TOPqUxUYS8vwLBgqJ7Fx0ViY1w/ue10CgaiQuP
# Ntq6TPmb/wrpNPgkNWcr4A245oyZ1uEi6vAnQj0llOZ0dFtq0Z4+7X6gMTN9vMvp
# e784cETRkPHIqzqKOghif9lwY1NNje6CbaUFEMFxBmoQtB1VM1izoXBm8qGCA00w
# ggI1AgEBMIH5oYHRpIHOMIHLMQswCQYDVQQGEwJVUzETMBEGA1UECBMKV2FzaGlu
# Z3RvbjEQMA4GA1UEBxMHUmVkbW9uZDEeMBwGA1UEChMVTWljcm9zb2Z0IENvcnBv
# cmF0aW9uMSUwIwYDVQQLExxNaWNyb3NvZnQgQW1lcmljYSBPcGVyYXRpb25zMScw
# JQYDVQQLEx5uU2hpZWxkIFRTUyBFU046OTIwMC0wNUUwLUQ5NDcxJTAjBgNVBAMT
# HE1pY3Jvc29mdCBUaW1lLVN0YW1wIFNlcnZpY2WiIwoBATAHBgUrDgMCGgMVALNy
# BOcZqxLB792u75w97U0X+/BDoIGDMIGApH4wfDELMAkGA1UEBhMCVVMxEzARBgNV
# BAgTCldhc2hpbmd0b24xEDAOBgNVBAcTB1JlZG1vbmQxHjAcBgNVBAoTFU1pY3Jv
# c29mdCBDb3Jwb3JhdGlvbjEmMCQGA1UEAxMdTWljcm9zb2Z0IFRpbWUtU3RhbXAg
# UENBIDIwMTAwDQYJKoZIhvcNAQELBQACBQDqQS2SMCIYDzIwMjQwNzE2MTczNDEw
# WhgPMjAyNDA3MTcxNzM0MTBaMHQwOgYKKwYBBAGEWQoEATEsMCowCgIFAOpBLZIC
# AQAwBwIBAAICG74wBwIBAAICEw0wCgIFAOpCfxICAQAwNgYKKwYBBAGEWQoEAjEo
# MCYwDAYKKwYBBAGEWQoDAqAKMAgCAQACAwehIKEKMAgCAQACAwGGoDANBgkqhkiG
# 9w0BAQsFAAOCAQEAJZxcDq3UkdKxGmaVq5uJbpjtw4w9kTUxRPnnWfkJc3hnZTpE
# FI4eOQfTyM4c9x0DXHLBSKVpDFbJT336OO5hyJcKkl7ogHIcLA9eHaxDvjv+coUF
# Z6JfMEkrzISDU2BGV3Eh91PaypJyOWHXkvyl59pfqsHvka4PFGyYMh+TeBsUzklk
# lVvlNB4YZsFNyJpydpT9zP3evbvLADXztDrkEY0uTlqRbhRjgf13eoSNpnyZLs+z
# 8gabFuyjyVTvuvERWk1H9z7W+Dvt0qHHNf7VpnMm+aBxl7CgWYIq+nP+ta/qjFv+
# sFg89pu306E3zHxAotURELfcFpfhcxgsN6UgvjGCBA0wggQJAgEBMIGTMHwxCzAJ
# BgNVBAYTAlVTMRMwEQYDVQQIEwpXYXNoaW5ndG9uMRAwDgYDVQQHEwdSZWRtb25k
# MR4wHAYDVQQKExVNaWNyb3NvZnQgQ29ycG9yYXRpb24xJjAkBgNVBAMTHU1pY3Jv
# c29mdCBUaW1lLVN0YW1wIFBDQSAyMDEwAhMzAAAB5y6PL5MLTxvpAAEAAAHnMA0G
# CWCGSAFlAwQCAQUAoIIBSjAaBgkqhkiG9w0BCQMxDQYLKoZIhvcNAQkQAQQwLwYJ
# KoZIhvcNAQkEMSIEIO/d8x9Mn/R4YCITBtVGf5UJQS9diKT2IUUxBk7+wFNvMIH6
# BgsqhkiG9w0BCRACLzGB6jCB5zCB5DCBvQQg5TZdDXZqhv0N4MVcz1QUd4RfvgW/
# QAG9AwbuoLnWc60wgZgwgYCkfjB8MQswCQYDVQQGEwJVUzETMBEGA1UECBMKV2Fz
# aGluZ3RvbjEQMA4GA1UEBxMHUmVkbW9uZDEeMBwGA1UEChMVTWljcm9zb2Z0IENv
# cnBvcmF0aW9uMSYwJAYDVQQDEx1NaWNyb3NvZnQgVGltZS1TdGFtcCBQQ0EgMjAx
# MAITMwAAAecujy+TC08b6QABAAAB5zAiBCCWb6Ur4G2181oQCc5/BvSQ76aaktVk
# 5NmByZFtNlISpTANBgkqhkiG9w0BAQsFAASCAgB39S5iMg0YoU/OhZDUKeX4LN7U
# KJ1BhBtGqoOxddlof74XVpa3llD6rh38sb54NVwQ4jWwz6wyPy6KqSNRyuSFGEGS
# +0959I966JtIFI/dOR1lD0BV3JT3Ct+5A+8P83oPVoOH03wqxfb8ejAb7GOS2ZPy
# xWACpymwOAcsDRsuEf8J0Sh0hDK6/X75YHL5lYWXNW8qGbO34Ij+BKpDPq7T0uhD
# x/Mu7626hlKCpbe6rU+uSTwRysRRHGuZurIJ9K8wkFwgi+rUdf7uzerzqr+N9wc3
# 2dW8jwh73XELt/w78+8/SSvJ5IxjkbaycRnJHFosrxsnjj78sy8bfEQeACoTHYEm
# t04MgoeAyEJTm8tQ7xrv5f3nGmuYXqW+RYJB1w4ZuFxcNueWjxWWqIDRtPObFG7K
# +OcfuTs9fCuahGH59fT90i2o9e+BEWkkgBOPUz8b2+njnGnZZJ9DlBnsEp6V5nF5
# i378xQCd3YrsI3lUGO52lyr8v3mzVSZ2o/gnTT95gtKsmwY4d+Meh+P0HgeUdqGJ
# 7x7/3PHfsPkCQUGMNkVlj8AO3VhkaWPo3T5QKqEb6cwxhpsUjyEtoBNZXsXY45TS
# OhxNOdlTcm6yTMqRwm+qoNSlbCcXVzDVNzJKZZjDCUHhLIFgLem8n2pRzVmrOpyR
# Rg6KW+KzOPM86rLBZg==
# SIG # End signature block
