# Headsup

A CLI tool that monitors subjects (games, TV shows, software, events) for release dates and answers using Claude AI for intelligent web search and analysis.

## Features

- **AI-Powered Search**: Uses Claude CLI for intelligent web search and source evaluation
- **Multiple Subject Types**:
  - `release` - Track release dates for games, movies, TV shows, software
  - `question` - Track answers to questions (e.g., "Who is the next James Bond?")
  - `recurring` - Track recurring events (e.g., Apple keynotes, E3)
- **Smart Notifications**: Only notifies when there's actual news (date announced, changed, confirmed)
- **Email Delivery**: SMTP-based notifications with optional digest mode
- **AI-Assisted Setup**: Intelligent subject identification when adding new items
- **Cron-Friendly**: Designed for scheduled background execution

## Installation

### From Source

```bash
git clone https://github.com/sudoremo/headsup.git
cd headsup
cargo build --release
```

The binary will be at `./target/release/headsup`.

### Prerequisites

- [Claude CLI](https://github.com/anthropics/claude-code) installed and authenticated
- SMTP server for email notifications

## Quick Start

```bash
# Initialize configuration
headsup init --email your@email.com

# Edit config to set up SMTP
headsup config edit

# Add a subject to track (AI-assisted)
headsup subjects add

# Run a check
headsup check

# Send a test email
headsup test-email
```

## Usage

### Commands

```
headsup [OPTIONS] [COMMAND]

Commands:
  check       Run a check for all subjects, or a specific one
  notify      Send pending notifications
  subjects    Manage monitored subjects
  config      Manage configuration
  state       Manage state
  history     View notification history
  init        Initialize config and state files
  test-email  Send a test email to verify SMTP configuration
  help        Print help information

Options:
  -v, --verbose     Increase log verbosity (can repeat: -vv)
  -q, --quiet       Suppress all output except errors
  --log-format      Output format: text (default) or json
  --config <PATH>   Use alternate config file
  --dry-run         Check but don't send emails or update state
  -h, --help        Print help
  -V, --version     Print version
```

### Managing Subjects

```bash
# List all subjects
headsup subjects list

# Add a new subject (interactive, AI-assisted)
headsup subjects add

# Remove a subject
headsup subjects remove gta6

# Enable/disable a subject
headsup subjects enable gta6
headsup subjects disable gta6

# Edit a subject
headsup subjects edit gta6
```

### Running Checks

```bash
# Check all enabled subjects
headsup check

# Check a specific subject
headsup check gta6

# Dry run (no emails, no state changes)
headsup check --dry-run

# Check but don't send emails (queue for later)
headsup check --no-notify

# Send queued notifications
headsup notify
```

### Configuration

```bash
# Show config (secrets redacted)
headsup config show

# Edit config in $EDITOR
headsup config edit

# Validate config
headsup config validate

# Show config file path
headsup config path
```

## Configuration File

Located at:
- **macOS**: `~/Library/Application Support/headsup/config.toml`
- **Linux**: `~/.config/headsup/config.toml`
- **Windows**: `%APPDATA%\headsup\config.toml`

### Example Configuration

```toml
[email]
to = "your@email.com"
from = "headsup@yourdomain.com"
smtp_host = "smtp.example.com"
smtp_port = 587
smtp_username = "user"
smtp_password_command = "op read 'op://Private/SMTP/password'"
smtp_timeout_seconds = 30
digest_mode = false

[claude]
command = "claude"
model = "sonnet"
max_searches_per_run = 20
timeout_seconds = 60
max_consecutive_failures = 3
total_run_timeout_seconds = 600

[settings]
log_level = "quiet"
log_format = "text"
imminent_threshold_days = 7
max_history_entries = 50

[[subjects]]
id = "550e8400-e29b-41d4-a716-446655440000"
key = "gta6"
name = "GTA 6"
type = "release"
category = "game"
search_terms = ["GTA 6 release date", "GTA VI launch date"]
notes = "Rockstar's next major release"
enabled = true

[[subjects]]
id = "550e8400-e29b-41d4-a716-446655440001"
key = "bond"
name = "Next James Bond Actor"
type = "question"
question = "Who will be the next James Bond actor after Daniel Craig?"
search_terms = ["next James Bond actor", "James Bond casting"]
enabled = true

[[subjects]]
id = "550e8400-e29b-41d4-a716-446655440002"
key = "apple"
name = "Apple Events"
type = "recurring"
event_name = "Apple Event"
search_terms = ["next Apple event", "Apple keynote", "WWDC"]
enabled = true
```

### Password Command

The `smtp_password_command` is executed to retrieve your SMTP password. Examples:

```toml
# 1Password CLI
smtp_password_command = "op read 'op://Private/SMTP/password'"

# macOS Keychain
smtp_password_command = "security find-generic-password -s 'smtp' -w"

# Environment variable (not recommended)
smtp_password_command = "echo $SMTP_PASSWORD"

# Pass password manager
smtp_password_command = "pass show email/smtp"
```

## Cron Setup

Run headsup daily at 9 AM:

```cron
0 9 * * * /usr/local/bin/headsup check --quiet 2>&1 | logger -t headsup
```

For digest mode (batch all notifications):

```cron
# Check throughout the day without sending
0 9,15,21 * * * /usr/local/bin/headsup check --no-notify --quiet
# Send digest once daily
0 22 * * * /usr/local/bin/headsup notify --digest --quiet
```

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | General error (config invalid, file not found) |
| 2 | Partial failure (some subjects failed) |
| 3 | All subjects failed |
| 4 | Email delivery failed |
| 5 | Timeout exceeded |

## Notification Triggers

### Release Type
- New release date announced
- Release date changed
- Date precision improved (e.g., "2025" → "Fall 2025" → "October 15, 2025")
- Release imminent (within 7 days)
- Confidence upgraded (rumor → official)

### Question Type
- Answer found
- Answer changed
- Confidence upgraded
- Answer confirmed as definitive

### Recurring Type
- Next event date announced
- Event date changed
- Event imminent
- Event happened (auto-resets to track next occurrence)

## State File

Located alongside the config file as `state.json`. Contains:
- Last check timestamps
- Known release dates/answers
- Notification history
- Failure tracking

The state file is protected by a lock file to prevent corruption from concurrent runs.

## Troubleshooting

### Claude not found
Ensure Claude CLI is installed and in your PATH:
```bash
which claude
claude --version
```

### SMTP connection failed
Test your SMTP settings:
```bash
headsup test-email
```

### Subject keeps failing
Check the history for error details:
```bash
headsup history <subject-key>
```

Subjects are auto-disabled after 3 consecutive failures. Re-enable with:
```bash
headsup subjects enable <subject-key>
```

### View debug output
```bash
headsup check -vv
```

## License

MIT

## Contributing

Contributions welcome! Please open an issue first to discuss major changes.
