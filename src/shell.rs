use crate::config::Config;
use crate::error::{JcvmError, Result};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shell {
    Bash,
    Zsh,
    Fish,
    PowerShell,
}

impl Shell {
    pub fn detect() -> Option<Self> {
        // Try SHELL environment variable first
        if let Ok(shell) = std::env::var("SHELL") {
            if shell.contains("zsh") {
                return Some(Shell::Zsh);
            } else if shell.contains("bash") {
                return Some(Shell::Bash);
            } else if shell.contains("fish") {
                return Some(Shell::Fish);
            }
        }

        // Check for PowerShell on Windows
        #[cfg(windows)]
        {
            return Some(Shell::PowerShell);
        }

        None
    }

    pub fn config_file(&self) -> Option<PathBuf> {
        let home = dirs::home_dir()?;

        match self {
            Shell::Bash => {
                // Try .bashrc first, then .bash_profile
                let bashrc = home.join(".bashrc");
                if bashrc.exists() {
                    Some(bashrc)
                } else {
                    Some(home.join(".bash_profile"))
                }
            }
            Shell::Zsh => Some(home.join(".zshrc")),
            Shell::Fish => Some(home.join(".config/fish/config.fish")),
            Shell::PowerShell => {
                #[cfg(windows)]
                {
                    Some(home.join("Documents/PowerShell/Microsoft.PowerShell_profile.ps1"))
                }
                #[cfg(not(windows))]
                None
            }
        }
    }

    pub fn init_script(&self, jcvm_dir: &str) -> String {
        match self {
            Shell::Bash | Shell::Zsh => format!(
                r#"
# JCVM - Java Configuration & Version Manager
export JCVM_DIR="{}"

# Add tool-specific bin directories to PATH
# Priority order: python, node, java (current is legacy java)
export PATH="$JCVM_DIR/alias/python/current/bin:$JCVM_DIR/alias/node/current/bin:$JCVM_DIR/alias/current/bin:$PATH"
export JAVA_HOME="$JCVM_DIR/alias/current"

# Wrapper function for jcvm use to update current shell environment
jcvm() {{
    local command="$1"
    shift
    
    case "$command" in
        use)
            # Call the actual jcvm binary and capture output
            local jcvm_output
            jcvm_output=$(command jcvm use "$@" 2>&1)
            local exit_code=$?
            
            # Print the output
            echo "$jcvm_output"
            
            # If successful, update the current shell environment
            if [ $exit_code -eq 0 ]; then
                export JAVA_HOME="$JCVM_DIR/alias/current"
                # Rebuild PATH with all tool paths
                # Remove old JCVM paths first
                local cleaned_path=$(echo "$PATH" | tr ':' '\n' | grep -v "$JCVM_DIR" | tr '\n' ':' | sed 's/:$//')
                export PATH="$JCVM_DIR/alias/python/current/bin:$JCVM_DIR/alias/node/current/bin:$JCVM_DIR/alias/current/bin:$cleaned_path"
            fi
            
            return $exit_code
            ;;
        *)
            # For all other commands, just pass through to the binary
            command jcvm "$command" "$@"
            ;;
    esac
}}

# Auto-switch JDK version on directory change
_jcvm_auto_switch() {{
    if [ -f ".java-version" ]; then
        local version=$(cat .java-version | tr -d '[:space:]')
        if [ -n "$version" ]; then
            jcvm use "$version" >/dev/null 2>&1
        fi
    fi
}}

# Hook for directory change
if [ -n "$ZSH_VERSION" ]; then
    autoload -U add-zsh-hook
    add-zsh-hook chpwd _jcvm_auto_switch
elif [ -n "$BASH_VERSION" ]; then
    _jcvm_cd() {{
        builtin cd "$@" && _jcvm_auto_switch
    }}
    alias cd='_jcvm_cd'
fi

# Run on shell startup
_jcvm_auto_switch
"#,
                jcvm_dir
            ),
            Shell::Fish => format!(
                r#"
# JCVM - Java Configuration & Version Manager
set -gx JCVM_DIR "{}"
set -gx PATH "$JCVM_DIR/alias/current/bin" $PATH
set -gx JAVA_HOME "$JCVM_DIR/alias/current"

# Wrapper function for jcvm use to update current shell environment
function jcvm
    set -l command $argv[1]
    set -e argv[1]
    
    switch $command
        case use
            # Call the actual jcvm binary and capture output
            set -l jcvm_output (command jcvm use $argv 2>&1)
            set -l exit_code $status
            
            # Print the output
            echo $jcvm_output
            
            # If successful, update the current shell environment
            if test $exit_code -eq 0
                set -gx JAVA_HOME "$JCVM_DIR/alias/current"
                # Remove old JAVA_HOME/bin from PATH if it exists
                set -l new_path
                for p in $PATH
                    if not string match -q "*/bin" $p; or not string match -q "*java*" $p
                        set new_path $new_path $p
                    end
                end
                set -gx PATH "$JAVA_HOME/bin" $new_path
            end
            
            return $exit_code
        case '*'
            # For all other commands, just pass through to the binary
            command jcvm $command $argv
    end
end

# Auto-switch JDK version on directory change
function _jcvm_auto_switch --on-variable PWD
    if test -f .java-version
        set version (cat .java-version | string trim)
        if test -n "$version"
            jcvm use "$version" >/dev/null 2>&1
        end
    end
end

# Run on shell startup
_jcvm_auto_switch
"#,
                jcvm_dir
            ),
            Shell::PowerShell => format!(
                r#"
# JCVM - Java Configuration & Version Manager
$env:JCVM_DIR = "{}"
$env:PATH = "$env:JCVM_DIR\alias\current\bin;$env:PATH"
$env:JAVA_HOME = "$env:JCVM_DIR\alias\current"

# Wrapper function for jcvm use to update current shell environment
function jcvm {{
    $command = $args[0]
    $restArgs = $args[1..($args.Length - 1)]
    
    if ($command -eq "use") {{
        # Call the actual jcvm binary and capture output
        $jcvmOutput = & jcvm.exe use $restArgs 2>&1 | Out-String
        $exitCode = $LASTEXITCODE
        
        # Print the output
        Write-Output $jcvmOutput
        
        # If successful, update the current shell environment
        if ($exitCode -eq 0) {{
            $env:JAVA_HOME = "$env:JCVM_DIR\alias\current"
            # Remove old JAVA_HOME\bin from PATH if it exists
            $pathParts = $env:PATH -split ';'
            $newPath = $pathParts | Where-Object {{ $_ -notmatch 'java.*\\bin$' }}
            $env:PATH = "$env:JAVA_HOME\bin;" + ($newPath -join ';')
        }}
        
        exit $exitCode
    }} else {{
        # For all other commands, just pass through to the binary
        & jcvm.exe $command $restArgs
    }}
}}

# Auto-switch JDK version on directory change
function Invoke-JcvmAutoSwitch {{
    if (Test-Path .java-version) {{
        $version = (Get-Content .java-version).Trim()
        if ($version) {{
            jcvm use $version 2>$null
        }}
    }}
}}

# Hook for directory change
$global:__JcvmLastDir = $PWD.Path
$ExecutionContext.InvokeCommand.LocationChangedAction = {{
    if ($global:__JcvmLastDir -ne $PWD.Path) {{
        $global:__JcvmLastDir = $PWD.Path
        Invoke-JcvmAutoSwitch
    }}
}}

# Run on shell startup
Invoke-JcvmAutoSwitch
"#,
                jcvm_dir
            ),
        }
    }

    pub fn install_hook(&self, config: &Config) -> Result<()> {
        let config_file = self.config_file().ok_or_else(|| {
            JcvmError::ShellError("Could not determine shell config file".to_string())
        })?;

        // Check if already installed
        if config_file.exists() {
            let contents = std::fs::read_to_string(&config_file)?;
            if contents.contains("JCVM_DIR") {
                return Err(JcvmError::ShellError(
                    "JCVM is already configured in your shell".to_string(),
                ));
            }
        }

        // Create config file if it doesn't exist
        if !config_file.exists() {
            if let Some(parent) = config_file.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&config_file, "")?;
        }

        // Append init script
        let init_script = self.init_script(config.jcvm_dir.to_str().unwrap());
        let mut contents = std::fs::read_to_string(&config_file)?;
        contents.push_str(&init_script);
        std::fs::write(&config_file, contents)?;

        Ok(())
    }

    pub fn use_command(&self, java_home: &str) -> String {
        match self {
            Shell::Bash | Shell::Zsh | Shell::Fish => format!(
                r#"export JAVA_HOME="{}"
export PATH="$JAVA_HOME/bin:$PATH""#,
                java_home
            ),
            Shell::PowerShell => format!(
                r#"$env:JAVA_HOME = "{}"
$env:PATH = "$env:JAVA_HOME\bin;$env:PATH""#,
                java_home
            ),
        }
    }
}

pub fn generate_activation_script(version_path: &str) -> Result<String> {
    let shell = Shell::detect().unwrap_or(Shell::Bash);
    Ok(shell.use_command(version_path))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shell_detection() {
        // This will vary based on the test environment
        let shell = Shell::detect();
        assert!(shell.is_some() || cfg!(windows));
    }

    #[test]
    fn test_init_script_generation() {
        let shell = Shell::Bash;
        let script = shell.init_script("/home/user/.jcvm");
        assert!(script.contains("JCVM_DIR"));
        assert!(script.contains("_jcvm_auto_switch"));
    }
}
