# Magnet: The Rust-Based GitHub Package Manager

**Magnet** is a powerful, cross-platform package manager for installing, managing, and downloading GitHub releases and repositories. Built with Rust's async capabilities, Magnet provides a seamless experience for working with GitHub-hosted software.

## Key Features

### Package Management
- **Automated Installation**: Install binaries from GitHub releases with a single command
- **Smart Binary Detection**: Automatically finds and extracts the correct binary for your platform
- **Version Control**: Install specific versions or always get the latest release
- **Update Management**: Update individual packages or all installed packages at once
- **Registry System**: Track installed packages with version history and metadata
- **Global and Local Installation**: Choose between system-wide or user-specific installations

### Repository Operations
- **Bulk Repository Download**: Clone all repositories from any GitHub user
- **Advanced Filtering**: Filter by language, stars, size, fork status, and regex patterns
- **Concurrent Downloads**: Configurable parallel processing for faster operations
- **Progress Tracking**: Real-time download progress and statistics
- **Automatic Branch Detection**: Fallback across common branch names for maximum compatibility

### Developer Experience
- **Cross-Platform**: Full support for Linux, macOS, and Windows
- **Colored Output**: Beautiful, readable terminal interface
- **Error Handling**: Robust error recovery with detailed error messages
- **Rate Limit Management**: Automatic GitHub API rate limit handling
- **Authentication Support**: Use GitHub tokens for higher rate limits

## Installation

### Quick Install

Download the latest release and install Magnet using itself:

```bash
magnet install naseridev/magnet
```

### From Source

```bash
git clone https://github.com/naseridev/magnet.git
```

```bash
cd magnet
```

```bash
cargo build --release
```

### Authentication (Recommended)

```bash
export GITHUB_TOKEN=your_personal_access_token
```

## Usage

### Installing Packages

```bash
# Install latest version
magnet install naseridev/cortex

# Install specific version
magnet install naseridev/cortex --version v1.0.0

# Install globally (system-wide)
magnet install naseridev/cortex --global

# Force reinstall
magnet install naseridev/cortex --force

# Install with authentication
magnet install naseridev/cortex --token ghp_your_token_here
```

### Managing Packages

```bash
# List installed packages
magnet list

# List with detailed information
magnet list --verbose

# Update specific package
magnet update naseridev/cortex

# Update all packages
magnet update

# Uninstall package
magnet uninstall naseridev/cortex

# Clean untracked binaries
magnet clean
```

### Repository Information

```bash
# Show repository details
magnet info naseridev/cortex

# Search repositories
magnet search cortex

# Search with limit
magnet search cortex --limit 20

# Search users
magnet search naseridev --by-user
```

### Bulk Repository Download

```bash
# Download all repositories from a user
magnet dump username

# Filter by language
magnet dump username --language rust

# Filter by stars
magnet dump username --min-stars 100

# Exclude forks
magnet dump username --only-original

# Filter by size (MB)
magnet dump username --max-size 50

# Filter by regex pattern
magnet dump username --regex "^api-.*"

# Concurrent downloads
magnet dump username --parallel 10

# Custom output directory
magnet dump username --output ./repos
```

### Advanced Filtering Examples

```bash
# High-quality Rust projects
magnet dump username --language rust --min-stars 500 --only-original

# Small documentation repos
magnet dump username --regex "(docs|config)" --max-size 10

# Large-scale parallel download
magnet dump username --parallel 15 --max-size 100

# Production codebases
magnet dump username --regex "(prod|production)" --only-original

# Microservices architecture
magnet dump username --regex "service-.*|.*-api$" --language go
```

## Command Reference

### Global Options

| Option | Short | Description |
|--------|-------|-------------|
| `--token` | `-t` | GitHub personal access token |
| `--verbose` | `-v` | Enable verbose output |

### Commands

#### `install`
Install a package from GitHub releases

| Option | Short | Description | Default |
|--------|-------|-------------|---------|
| `package` | - | Package in format: owner/repo | Required |
| `--version` | `-V` | Specific version tag | Latest |
| `--global` | `-g` | Install system-wide | false |
| `--force` | `-f` | Force reinstall | false |

#### `uninstall`
Remove an installed package

| Option | Short | Description | Default |
|--------|-------|-------------|---------|
| `package` | - | Package in format: owner/repo | Required |
| `--global` | `-g` | Uninstall from global directory | false |

#### `list`
List all installed packages

| Option | Short | Description | Default |
|--------|-------|-------------|---------|
| `--verbose` | `-v` | Show detailed information | false |

#### `update`
Update installed packages

| Option | Short | Description | Default |
|--------|-------|-------------|---------|
| `package` | - | Package to update (all if omitted) | None |
| `--global` | `-g` | Update global packages | false |

#### `info`
Display repository information

| Option | Short | Description | Default |
|--------|-------|-------------|---------|
| `package` | - | Package in format: owner/repo | Required |

#### `search`
Search for packages or users

| Option | Short | Description | Default |
|--------|-------|-------------|---------|
| `query` | - | Search query | Required |
| `--limit` | `-l` | Maximum results | 10 |
| `--by-user` | `-u` | Search users instead of repos | false |

#### `clean`
Clean up untracked files

| Option | Short | Description | Default |
|--------|-------|-------------|---------|
| `--global` | `-g` | Clean global directory | false |

#### `dump`
Bulk download repositories

| Option | Short | Description | Default |
|--------|-------|-------------|---------|
| `username` | - | GitHub username | Required |
| `--language` | `-l` | Filter by language | None |
| `--min-stars` | `-s` | Minimum star count | 0 |
| `--only-original` | `-o` | Exclude forks | false |
| `--regex` | `-r` | Filter by regex pattern | None |
| `--max-size` | `-m` | Maximum size in MB | None |
| `--parallel` | `-p` | Concurrent downloads | 3 |
| `--output` | `-d` | Output directory | username |

## Installation Paths

### Local Installation
- **Linux/macOS**: `~/.magnet/bin`
- **Windows**: `%USERPROFILE%\.magnet\bin`

### Global Installation
- **Linux/macOS**: `/usr/local/bin`
- **Windows**: `C:\Program Files\magnet\bin`

### Registry Files
- **Local**: `~/.magnet/registry.json`
- **Global**: `/usr/local/magnet/registry.json` (Unix) or `C:\Program Files\magnet\registry.json` (Windows)

## Platform Support

### Supported Architectures
- x86_64 (amd64)
- aarch64 (arm64)
- x86 (i686)

### Supported Archive Formats
- `.tar.gz` / `.tgz`
- `.zip`
- `.exe` (Windows)
- Raw binaries

### Automatic Platform Detection
Magnet automatically detects your OS and architecture to download the correct binary:
- **Linux**: Prefers musl static binaries for maximum compatibility
- **macOS**: Supports both Intel and Apple Silicon
- **Windows**: Handles both MSVC and GNU toolchains

## Performance

### Scalability
- **Concurrent Downloads**: Up to 50+ parallel operations
- **Smart Binary Scoring**: Intelligent asset selection with 100+ scoring criteria
- **Network Efficiency**: Persistent connections with streaming downloads
- **Memory Optimization**: Minimal footprint with efficient buffering

### Typical Performance
- **Small packages** (< 1MB): ~0.5 seconds
- **Medium packages** (1-10MB): ~2-5 seconds
- **Large packages** (10-100MB): ~10-30 seconds

### Rate Limits
- **Authenticated**: 5000 requests/hour
- **Unauthenticated**: 60 requests/hour

## Error Handling

### Automatic Retry
- Up to 3 retry attempts with exponential backoff
- Automatic branch fallback (main, master, develop, trunk)
- Graceful rate limit handling

### Timeout Protection
- 5-minute timeout for downloads
- 2GB maximum download size
- Safe cancellation on size limit exceeded

## Real-World Use Cases

### Development Workflow
```bash
# Set up development tools
magnet install sharkdp/fd
magnet install sharkdp/bat
magnet install junegunn/fzf
magnet install naseridev/cortex

# Update all tools
magnet update
```

### Code Backup
```bash
# Backup all repositories
magnet dump your-username --parallel 10

# Backup only important projects
magnet dump your-username --min-stars 10 --only-original
```

### Research and Analysis
```bash
# Collect Rust projects
magnet dump username --language rust --min-stars 50

# Analyze popular Python libraries
magnet dump username --language python --min-stars 1000
```

## Troubleshooting

### Rate Limit Issues
```bash
export GITHUB_TOKEN=your_token
magnet install package --token $GITHUB_TOKEN
```

### Download Failures
```bash
magnet install package --verbose
magnet install package --parallel 2
```

### Permission Issues
```bash
# Install locally instead of globally
magnet install package

# Or use sudo for global installation
sudo magnet install package --global
```

### PATH Not Updated
**Windows**: Restart your terminal or run:
```powershell
$env:Path = [System.Environment]::GetEnvironmentVariable("Path","User")
```

**Unix**: Source your shell configuration:
```bash
source ~/.bashrc  # or ~/.zshrc
```

## Contributing

Contributions are welcome! Focus areas:
- Enhanced binary detection algorithms
- Additional archive format support
- Performance optimizations
- Cross-platform compatibility improvements
- Better error messages and user guidance

## Disclaimer

This tool is intended for legitimate software installation and repository management. Users are responsible for:
- Compliance with GitHub's Terms of Service
- Respecting rate limits and API usage policies
- Following applicable copyright and licensing laws
- Proper authentication and security practices

Always use a GitHub personal access token to avoid rate limiting and ensure proper API usage attribution.