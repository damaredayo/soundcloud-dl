# soundcloud-dl

An easy, fast and efficient tool written in Rust for downloading songs from SoundCloud.

## Getting Started

### Getting Your OAuth Token

1. Log into SoundCloud in your browser
2. Open Developer Tools (F12)
3. Go to the Network tab
4. Reload the page
5. Filter for a request to `https://api-v2.soundcloud.com` in the list
6. Copy the `Authorization` header value from the request headers
7. Use the copied value as the OAuth token

## Installation

### Pre-built Binaries

Head over to the [releases page](http://github.com/damaredayo/soundcloud-dl/releases) and download the latest binary for your platform.

### Cargo

```bash
cargo install --git https://github.com/damaredayo/soundcloud-dl
```

### Building from Source

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

# Download a playlist
soundcloud-dl playlist https://soundcloud.com/user/playlist

# Download liked tracks
soundcloud-dl likes --limit 50 --skip 10
```

### Help

```text
An easy, fast and efficient tool written in Rust for downloading songs from SoundCloud.

Usage: soundcloud-dl [OPTIONS] [COMMAND]

Options:
  -a, --auth <AUTH>      Your Soundcloud OAuth token (if not provided, will use stored token)
  -t, --save-token       Save the provided OAuth token for future use
      --clear-token      Clear the stored OAuth token
      --ffmpeg-path <FFMPEG_PATH>  FFmpeg binary path (if not provided, will use `ffmpeg` from PATH or download it)
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
  playlist
    <URL>  URL of the playlist to download
  help   Print this message or the help of the given subcommand
```

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
