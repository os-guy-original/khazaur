# Khazaur Retry Mechanism

## Overview

Khazaur includes a robust HTTP retry mechanism with exponential backoff to handle transient network and server errors gracefully.

## Retryable HTTP Status Codes

The following HTTP status codes trigger automatic retries:

- **408 Request Timeout**: Request took too long
- **429 Too Many Requests**: Rate limiting
- **500 Internal Server Error**: Server-side error
- **502 Bad Gateway**: Gateway or proxy error (like your case!)
- **503 Service Unavailable**: Server temporarily unavailable
- **504 Gateway Timeout**: Upstream timeout

## Retry Configuration

### Default Settings

```rust
max_retries: 3                    // Total of 4 attempts (1 initial + 3 retries)
initial_backoff_ms: 500           // Start with 500ms delay
max_backoff_ms: 10000             // Cap at 10 seconds
backoff_multiplier: 2.0           // Double the delay each retry
```

### Backoff Strategy

Exponential backoff with the following progression:
- Attempt 1: Immediate
- Attempt 2: Wait 500ms
- Attempt 3: Wait 1000ms (500ms × 2)
- Attempt 4: Wait 2000ms (1000ms × 2)

Total maximum wait time: ~3.5 seconds across all retries.

## What Gets Retried

### AUR API Calls

All AUR RPC API calls include retry logic:

1. **Package Search** (`khazaur -Ss <query>`)
   - Retries on transient failures
   - Logs retry attempts to help debug issues

2. **Package Info** (`khazaur -Si <package>`)
   - Retries on server errors
   - Clear error messages after exhausting retries

3. **Package Download** (PKGBUILD tarballs)
   - Critical for package installation
   - Most likely to encounter 502 errors
   - Automatically retries download failures

### Network Errors

In addition to HTTP status codes, network-level errors are also retried:
- Connection timeouts
- DNS resolution failures
- Connection refused
- Other transport-level errors

## User Experience

### Transparent Retries

When a retry occurs, you'll see log messages (with `RUST_LOG=debug` or `RUST_LOG=warn`):

```
WARN: Received retryable status 502 Bad Gateway on attempt 1, retrying in 500ms...
WARN: Received retryable status 502 Bad Gateway on attempt 2, retrying in 1000ms...
```

### Failure After Retries

If all retries fail, you'll see a clear error:

```
Error: Failed to download paru after retries: <detailed error>
```

## Example Scenarios

### Scenario 1: Temporary AUR Outage

```bash
$ khazaur -S paru

# Behind the scenes:
# Attempt 1: 502 Bad Gateway → Wait 500ms
# Attempt 2: 502 Bad Gateway → Wait 1000ms
# Attempt 3: 200 OK → Success!

Installing Packages
→ Processing 1 AUR packages
✓ paru installed successfully
```

### Scenario 2: Persistent Failure

```bash
$ khazaur -S paru

# Behind the scenes:
# Attempt 1: 502 Bad Gateway → Wait 500ms
# Attempt 2: 502 Bad Gateway → Wait 1000ms
# Attempt 3: 502 Bad Gateway → Wait 2000ms
# Attempt 4: 502 Bad Gateway → Give up

Error: Failed to download paru after retries: HTTP 502 Bad Gateway
```

## Implementation Details

### Code Location

The retry logic is implemented in [`src/aur/retry.rs`](file:///home/sd-v/Documents/khazaur/src/aur/retry.rs).

### Key Functions

- `is_retryable_status(status: StatusCode) -> bool`: Determines if a status code warrants a retry
- `retry_request<F>(operation: F, config: &RetryConfig) -> Result<Response>`: Generic retry wrapper

### Integration Points

Retry logic is integrated in:
- `AurClient::search()` - Package searches
- `AurClient::info()` - Package info queries
- `AurClient::download_snapshot()` - PKGBUILD downloads

## Benefits

### Reliability
- Automatically recovers from temporary AUR server issues
- Handles network hiccups without user intervention

### User Experience
- No manual retries needed
- Transparent recovery from most transient failures
- Clear feedback when permanent failures occur

### Performance
- Smart backoff prevents hammering failing servers
- Respects rate limits (429 errors)
- Minimal delay on success (no unnecessary waiting)

## Customization (Future)

The retry configuration could be made customizable in the future through `~/.config/khazaur/config.toml`:

```toml
[retry]
max_retries = 5
initial_backoff_ms = 1000
max_backoff_ms = 30000
backoff_multiplier = 2.5
```

## Testing

Run the included unit tests:

```bash
cargo test retry
```

Tests verify:
- Correct identification of retryable status codes
- Non-retryable codes are not retried (404, 403, etc.)
