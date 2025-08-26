# Magnet: Industrial Strength GitHub Repository Scraper

**Magnet** is a high-performance, parallel GitHub repository scraper designed for critical scenarios requiring rapid, large-scale repository acquisition and analysis. Built with Rust's async capabilities, Magnet provides robust, fault-tolerant repository collection with advanced filtering and concurrent download mechanisms.

## Critical Use Cases

- **Emergency Code Recovery**: Rapid repository backup during critical infrastructure incidents
- **Disaster Response**: Quick preservation of essential codebases during organizational disruptions
- **Research Data Collection**: Systematic acquisition of repositories for computational research
- **Compliance Auditing**: Bulk repository analysis for security and compliance assessments
- **Code Migration**: Efficient transfer of multiple repositories during platform migrations
- **Forensic Analysis**: Comprehensive repository collection for digital forensics investigations

## Key Features

### Advanced Filtering System
- **Language-based filtering**: Target specific programming languages
- **Star threshold filtering**: Focus on repositories with minimum popularity metrics
- **Size constraints**: Control repository size limits (in MB)
- **Fork exclusion**: Option to retrieve only original repositories
- **Regex pattern matching**: Sophisticated repository name filtering using regular expressions
- **Multi-criteria filtering**: Combine multiple filters for precise targeting

### High-Performance Architecture
- **Concurrent downloads**: Configurable parallel processing (default: 3 concurrent operations)
- **Async I/O operations**: Non-blocking network and file system operations
- **Intelligent branch detection**: Automatic fallback across common branch names (main, master, develop, trunk)
- **Robust error handling**: Graceful failure recovery with detailed error reporting
- **Progress tracking**: Real-time download progress and statistics

### Enterprise-Grade Reliability
- **Timeout management**: 5-minute timeout protection for large repositories
- **User-agent spoofing**: Mimics standard curl requests to avoid rate limiting
- **Automatic retry logic**: Multiple branch attempts for maximum success rate
- **Directory structure preservation**: Maintains original repository organization
- **Size calculation**: Accurate downloaded content measurement

## Installation

### Prerequisites
- Rust 1.70+ with Cargo
- Internet connectivity for GitHub API access

### Dependencies
```toml
[dependencies]
clap = "4.0"
regex = "1.0"
reqwest = { version = "0.11", features = ["json"] }
serde_json = "1.0"
tokio = { version = "1.0", features = ["full"] }
zip = "0.6"
```

### Build Instructions
```bash
git clone https://github.com/naseridev/magnet.git
cd magnet
cargo build --release
```

## Usage

### Basic Repository Scraping
```bash
# Download all repositories for a user
./magnet username

# Download with parallel processing
./magnet username --parallel 5
```

### Advanced Filtering
```bash
# Language-specific repositories
./magnet username --language rust

# High-quality repositories only
./magnet username --min-stars 100

# Size-constrained downloads
./magnet username --max-size 50

# Original repositories only (exclude forks)
./magnet username --only-original

# Pattern-based filtering
./magnet username --regex "^api-.*"
```

### Complex Filtering Scenarios
```bash
# Critical infrastructure code collection
./magnet username --language go --min-stars 500 --only-original --max-size 100

# Emergency backup with specific patterns
./magnet username --regex "^(core|critical|prod)" --parallel 10

# Research data collection
./magnet username --language python --min-stars 50 --max-size 200 --parallel 8

# Security-focused repository collection
./magnet username --regex "(security|auth|crypto)" --language rust --min-stars 10

# Microservices architecture backup
./magnet username --regex "service-.*|.*-api$" --only-original --max-size 50

# Popular JavaScript libraries
./magnet username --language javascript --min-stars 1000 --parallel 5

# Documentation and configuration repos
./magnet username --regex "(docs|config|setup)" --max-size 10
```

### Real-World Scenarios

#### Emergency Code Recovery
```bash
# Backup critical repositories during infrastructure incident
./magnet company-username --only-original --parallel 15 --max-size 500

# Focus on production-related code
./magnet username --regex "(prod|production|deploy)" --parallel 8
```

#### Research and Analysis
```bash
# Machine learning repositories for analysis
./magnet researcher --language python --regex "(ml|ai|neural|deep)" --min-stars 20

# Web framework comparison study
./magnet username --language go --regex "(framework|web|http)" --min-stars 100
```

#### Compliance and Auditing
```bash
# Collect repositories for security audit
./magnet organization --language java --min-stars 5 --only-original

# Focus on repositories with specific naming conventions
./magnet username --regex "^[a-z]+-[a-z]+$" --max-size 100
```

## Command Line Interface

| Option | Short | Description | Default |
|--------|-------|-------------|---------|
| `username` | - | Target GitHub username (required) | - |
| `--language` | `-l` | Filter by programming language | None |
| `--min-stars` | `-s` | Minimum star count threshold | 0 |
| `--max-size` | `-m` | Maximum repository size (MB) | None |
| `--only-original` | `-o` | Exclude forked repositories | false |
| `--regex` | `-r` | Repository name regex pattern | None |
| `--parallel` | `-p` | Concurrent download count | 3 |

## Performance Characteristics

### Scalability Metrics
- **Concurrent Operations**: Up to 50+ parallel downloads (system-dependent)
- **Network Efficiency**: Persistent HTTP connections with connection pooling
- **Memory Usage**: Minimal memory footprint with streaming downloads
- **Error Recovery**: Individual failure isolation prevents cascade failures

### Typical Performance
- **Small repositories** (< 1MB): ~0.5 seconds per repository
- **Medium repositories** (1-10MB): ~2-5 seconds per repository
- **Large repositories** (10-100MB): ~10-30 seconds per repository
- **Network dependent**: Performance scales with available bandwidth

## Output Structure

```
username/
├── repository-1/
│   ├── src/
│   ├── README.md
│   └── ...
├── repository-2/
│   ├── lib/
│   ├── tests/
│   └── ...
└── repository-n/
    └── ...
```

## Error Handling

### Robust Failure Management
- **Network timeouts**: 300-second timeout with graceful degradation
- **Rate limiting**: Automatic backoff and retry mechanisms
- **Invalid repositories**: Individual failure isolation
- **Disk space**: Graceful handling of storage constraints
- **Permission errors**: Clear error reporting for access issues

### Status Reporting
```
[1/10] repository-name (1.2 MB)
[2/10] another-repo (500 KB)
[3/10] failed-repo FAILED: Network timeout
...

Results:
Downloaded: 8
Failed: 2
Total size: 150 MB
Time: 45.32s
Speed: 3.3 MB/s
```

## Technical Implementation

### Architecture Overview
- **Async Runtime**: Tokio-based asynchronous execution
- **HTTP Client**: reqwest with connection pooling and timeout management
- **Concurrency Control**: Semaphore-based parallel execution limiting
- **Progress Tracking**: Thread-safe progress reporting with Arc<Mutex<T>>
- **ZIP Processing**: Efficient archive extraction with path sanitization

### Security Considerations
- **Path Traversal Protection**: Sanitized extraction paths
- **Resource Limits**: Configurable size and timeout constraints
- **Rate Limiting Compliance**: Respectful API usage patterns
- **User-Agent Management**: Standard browser identification

## Limitations and Considerations

### API Constraints
- **Rate Limiting**: Subject to GitHub API rate limits (5000 requests/hour for authenticated users)
- **Repository Access**: Limited to publicly accessible repositories
- **Branch Availability**: Attempts multiple common branch names for maximum compatibility

### System Requirements
- **Disk Space**: Sufficient storage for target repositories
- **Network Bandwidth**: Stable internet connection for optimal performance
- **System Resources**: RAM and CPU capacity for concurrent operations

## Contributing

This tool is designed for critical use cases requiring reliability and performance. Contributions should focus on:
- Enhanced error handling and recovery mechanisms
- Performance optimizations for large-scale operations
- Additional filtering capabilities for specialized use cases
- Improved progress reporting and monitoring features

## Disclaimer

This tool is intended for legitimate repository collection and analysis purposes. Users are responsible for compliance with GitHub's Terms of Service, rate limiting policies, and applicable copyright laws. The tool respects repository access permissions and does not bypass any security mechanisms.