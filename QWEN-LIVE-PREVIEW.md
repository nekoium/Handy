# Qwen live preview

This experimental Handy fork shows tentative Qwen text while you dictate. Final dictation uses Handy’s usual transcription and background long-form chunks. Preview is optional and disabled unless you launch with the preview script.

Preview repeatedly transcribes the latest eight seconds of audio. It can revise or replace the entire visible hypothesis; older text scrolls out of that audio window. It starts after about two seconds of audio, then tries again after at least three seconds of new audio when background work permits. This repeats inference and uses extra compute. It does not provide cached incremental streaming.

## Build from source on Windows x64

Install Git, Rust, Bun, Visual Studio 2022 Build Tools with C++ support, CMake, and the Vulkan SDK. See [BUILD.md](BUILD.md) for platform prerequisites. Set `VULKAN_SDK` to your SDK directory and make `git`, `cmake`, `cargo`, and `bun` available in PowerShell.

Clone this experiment’s branch, then build and launch from its root:

```powershell
git clone --branch qwen-live-preview --single-branch https://github.com/nekoium/Handy.git
cd Handy
.\scripts\setup-qwen-preview.ps1
.\scripts\run-qwen-preview.ps1
```

Setup downloads an unchanged [transcribe.cpp checkout](https://github.com/handy-computer/transcribe.cpp/tree/adfc50210a3f026a655a3e19937d90ef2bf0728c) pinned to `adfc50210a3f026a655a3e19937d90ef2bf0728c` into ignored `.qwen-preview/upstream/`, builds its Vulkan runtime, downloads the public Silero VAD model if needed, and builds Handy’s frontend and portable debug application. Cargo’s checked-in lockfile and relative source overrides use that checkout’s unchanged Rust bindings. Run setup before invoking Cargo directly. Setup requires a clean backend checkout and does not apply patches or require a second fork. Allow several gigabytes for dependencies and build output.

To fetch and verify only the backend source, use `.\scripts\setup-qwen-preview.ps1 -PrepareOnly`. This skips compilation. The full setup builds from source; this branch provides no prebuilt installer.

The build and portable application data live under `.qwen-preview/`. The launcher uses a separate application identifier and portable settings. It does not replace an installed Handy application. Close other Handy instances before using the prototype to avoid competing global shortcuts.

In the portable app, download or import a Qwen3-ASR model, select the **Live** overlay, and choose your spoken languages. Keep **Long-form chunking** enabled to decode authoritative chunks during longer recordings. Press your normal dictation shortcut and speak. The visible text is tentative; the final pasted text may differ.

To run without preview:

```powershell
.\scripts\run-qwen-preview.ps1 -NoPreview
```

Close and restart the portable instance when switching modes. Advanced source users can set `HANDY_QWEN_LIVE_PREVIEW=1`; this flag is process-local and applies only to Qwen with the Live overlay.

## Observed tradeoff

Warm production-worker replays on Windows with Qwen3-ASR 1.7B Q8_0 and a Radeon 8060S produced these samples. Each length had one measured baseline/preview pair after one warmup pair. Both modes used the same authoritative chunk paths and produced identical final text.

| Audio length | Added stop wait | First tentative text | Total worker-call time versus baseline |
| -----------: | --------------: | -------------------: | -------------------------------------: |
|   10 seconds |          0.9 ms |              2.106 s |                                  2.40× |
|   30 seconds |         34.8 ms |              2.107 s |                                  3.28× |
|  120 seconds |         70.8 ms |              2.107 s |                                  3.87× |

These are replay samples, not latency guarantees or full application timings. The selected stops had no preview call in flight; another stop phase can wait for the remainder of an accepted call. The largest observed preview call was 387.9 ms, which is not a universal bound. Repeated inference can increase power use and heat. The rolling eight-second window limits retained preview audio, not total inference cost or GPU memory.

## Limitations

- Windows x64 with Qwen3-ASR 1.7B Q8_0 and AMD Vulkan is the measured configuration. Qwen 0.6B, other quantizations, platforms, and GPUs are unmeasured.
- Tentative text is a rolling hypothesis, not a growing transcript. Window boundaries can lose context, and mixed-language hypotheses can omit earlier words. No accuracy improvement is claimed.
- Background authoritative work takes priority. Slow devices can produce sparse updates. Cold model/shader initialization can delay the first text.
- Stop discards further preview opportunities. One already submitted batch call may finish before final decoding. Cancel uses Handy’s existing worker termination rather than native stream cancellation.
- Recognition and dictation remain local. Setup and model downloads require network access; audio and models are not included in this source checkout.
- Microphone capture, overlay rendering, and global shortcuts have not been validated as part of the automated replay measurements. This remains an experimental source build.

Upstream downloads linked in the main README do not include this prototype. This personal fork was developed with AI coding assistance and has not been accepted or endorsed by upstream Handy. It retains [Handy’s MIT license](LICENSE).
