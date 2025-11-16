# WebXR Development Guide

## WebXR Requirements

WebXR **requires HTTPS** for security reasons. The only exception is `localhost` which works over HTTP.

## Methods to Access WebXR from Your Headset

### Option 1: Cloudflare Tunnel (Easiest, No Setup)

1. Install cloudflared (one-time setup):
   ```bash
   # Automatic installation
   just init-webxr-tunnel

   # Or manually:
   # Windows: winget install Cloudflare.cloudflared
   # macOS: brew install cloudflared
   # Linux: https://github.com/cloudflare/cloudflared/releases
   ```

2. Run with automatic tunnel:
   ```bash
   just run-webxr-tunnel
   ```

3. Copy the generated HTTPS URL (e.g., `https://xyz.trycloudflare.com`) and open it in your VR headset's browser

4. Click the "Enter VR" button

**Pros**: Zero configuration, automatic HTTPS, works anywhere
**Cons**: Requires internet, URL changes each time

### Option 2: mkcert + Local Network (Best for Development)

1. Install mkcert:
   ```bash
   # Windows (Chocolatey)
   choco install mkcert

   # Windows (Scoop)
   scoop bucket add extras
   scoop install mkcert

   # Or download from: https://github.com/FiloSottile/mkcert
   ```

2. Find your local IP:
   ```bash
   just webxr-ip
   # Look for something like: 192.168.1.100
   ```

3. Create certificates:
   ```bash
   mkcert -install
   mkcert localhost 192.168.1.100  # Use your actual IP
   ```
   This creates `localhost+1.pem` and `localhost+1-key.pem`

4. Serve with HTTPS:
   ```bash
   just run-webxr-https localhost+1.pem localhost+1-key.pem
   ```

5. Install the CA on your headset:
   - Get the CA cert: `mkcert -CAROOT` shows location
   - Transfer `rootCA.pem` to your headset
   - Install it in your VR browser settings

6. Access `https://192.168.1.100:8443` from your VR browser

**Pros**: Fast, no internet needed, professional setup
**Cons**: Requires cert installation on headset

### Option 3: ngrok (Alternative Tunnel)

1. Install ngrok: https://ngrok.com/download

2. Start your dev server:
   ```bash
   just run-webxr-network
   ```

3. Create tunnel:
   ```bash
   ngrok http 8080
   ```

4. Use the HTTPS URL shown in ngrok dashboard

**Pros**: Easy, automatic HTTPS
**Cons**: Free tier has limitations, requires account

### Option 4: Meta Quest Developer Mode (Quest Headsets Only)

If using Meta Quest, you can use the built-in browser in developer mode:

1. Enable developer mode on your Quest
2. Connect Quest to PC via USB
3. Use `adb reverse` to forward the port:
   ```bash
   adb reverse tcp:8080 tcp:8080
   ```
4. Access `http://localhost:8080` in the Quest browser

**Note**: This only works with HTTP on localhost through ADB, not over WiFi

## Recommended Workflow

For quick testing:
```bash
# Terminal 1
just run-webxr-network

# Terminal 2
cloudflared tunnel --url http://localhost:8080
```

For serious development:
```bash
# One-time setup
mkcert -install
mkcert localhost $(ipconfig | grep "IPv4" | head -1 | awk '{print $NF}')

# Every session
just run-webxr-https localhost+1.pem localhost+1-key.pem
```

## Troubleshooting

### "WebXR Not Supported"
- Ensure you're using HTTPS (or localhost via ADB)
- Check that your VR browser supports WebXR
- Try Meta Quest Browser or Firefox Reality

### Certificate Errors
- Install the mkcert root CA on your headset
- Ensure the cert includes your current IP address
- Regenerate certs if your IP changed

### Connection Refused
- Check firewall settings
- Ensure headset is on same WiFi network
- Verify server is listening on `0.0.0.0`, not just `127.0.0.1`

### Performance Issues
- Use release build: `trunk serve --release --features webxr`
- Enable WebGL multisampling in your headset settings
- Check browser console for errors

## Commands Summary

```bash
# Install cloudflared (one-time)
just init-webxr-tunnel

# Run with Cloudflare tunnel (easiest!)
just run-webxr-tunnel

# Development (local only)
just run-webxr

# Network access (HTTP, won't work for WebXR over network)
just run-webxr-network

# HTTPS with custom cert
just run-webxr-https cert.pem key.pem

# Show your IP
just webxr-ip

# Get HTTPS setup instructions
just init-webxr-https
```
