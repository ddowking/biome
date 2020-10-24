# Test Supervisor package cleanup using the `--keep-latest-packages` option

$pkg="biome/bio-sup"

Describe "Supervisor package cleanup" {
    It "loads old Supervisor versions" {
        bio pkg install "biome/bio-sup/1.5.30" --channel unstable
        bio pkg install "biome/bio-sup/1.5.42" --channel unstable
        bio pkg install "biome/bio-sup/1.5.50" --channel unstable
        bio pkg install "biome/bio-sup/1.5.60" --channel unstable
        Wait-CommandLinesOfOutput "bio pkg list $pkg" 4
    }

    Context "start the Supervisor without package cleanup" {
        $supLog = New-SupervisorLogFile("start_the_supervisor_without_package_cleanup")
        Start-Supervisor -LogFile $supLog -Timeout 45 | Out-Null

        It "does not remove old Supervisor packages" {
            Wait-CommandLinesOfOutput "bio pkg list $pkg" 5
        }
        Stop-Supervisor
        Start-Sleep 3 # Wait for the supervisor to actually stop
    }

    Context "start the Supervisor with package cleanup" {
        $supLog = New-SupervisorLogFile("start_the_supervisor_with_package_cleanup")
        $expected = bio pkg list biome/bio-sup | Select-Object -Last 2
        Start-Supervisor -LogFile $supLog -Timeout 45 -SupArgs @( `
                "--keep-latest-packages=2"
        ) | Out-Null

        It "removes old Supervisor packages" {
            Wait-CommandLinesOfOutput "bio pkg list $pkg" 2
            bio pkg list $pkg | Should -Be $expected
        }
    }

    AfterAll {
        Stop-Supervisor
    }
}
