# soundcloud-dl

A fast and efficient Rust tool for downloading your liked tracks from SoundCloud.

## Prerequisites

- Rust 1.70 or higher
- FFmpeg installed and available in PATH
- SoundCloud OAuth token

### Getting Your OAuth Token

1. Log into SoundCloud in your browser
2. Open Developer Tools (F12)
3. Go to the Network tab
4. Reload the page
5. Filter for a request to `https://api-v2.soundcloud.com` in the list
6. Copy the `Authorization` header value from the request headers
7. Use the copied value as the OAuth token

## Installation

```bash
cargo install --git https://github.com/damaredayo/soundcloud-dl
```

Or clone the repository and build it manually:

```bash
git clone https://github.com/damaredayo/soundcloud-dl
cd soundcloud-dl
cargo build --release
```

## Example Usage

```bash
# Download track without saving token
soundcloud-dl -a YOUR_TOKEN track https://soundcloud.com/user/track

# Save token
soundcloud-dl --auth "OAuth 2-123456-133742069-xDxDxDxDxDxDxD" --save-token

# Download a single track
soundcloud-dl track https://soundcloud.com/user/track

# Download liked tracks
soundcloud-dl likes --limit 50 --skip 10

# Use saved token to download likes
soundcloud-dl likes --limit 20
```

### Help

```text
A tool to download SoundCloud liked tracks

Usage: soundcloud-dl [OPTIONS] [COMMAND]

Options:
  -a, --auth <AUTH>      Your Soundcloud OAuth token (if not provided, will use stored token)
  -t, --save-token       Save the provided OAuth token for future use
      --clear-token      Clear the stored OAuth token
  -o, --output <OUTPUT>  Output directory for downloaded files [default: .]
  -h, --help             Print help
  -V, --version          Print version

Commands:
  track
    <URL>  URL of the track to download
  likes
    -s, --skip <SKIP>              Number of likes to skip [default: 0]
    -l, --limit <LIMIT>            Maximum number of likes to download [default: 10]
        --chunk-size <CHUNK_SIZE>  Number of likes to download in each chunk [default: 25]
    -h, --help
  help   Print this message or the help of the given subcommand
```

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
