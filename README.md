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

## Usage

```bash
soundcloud-dl -a YOUR_OAUTH_TOKEN [OPTIONS]
```

### Options

```text
-a, --auth <TOKEN>      Your SoundCloud OAuth token (required)
-o, --output <DIR>      Output directory [default: current directory]
-s, --offset <NUMBER>   Number of tracks to skip [default: 0]
-l, --limit <NUMBER>    Number of tracks to download [default: 10]
    --chunk-size <SIZE> API request chunk size [default: 25]
-h, --help             Print help
-V, --version          Print version
```

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
