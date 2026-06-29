"""Kokoro TTS pipeline — local text-to-speech synthesis.

Ports logic from video_director_kokoro.py (Kokoro KPipeline integration)
and tts_manager.py (KokoroTTS provider).
"""

from __future__ import annotations

import asyncio
import base64
import io
import logging
import uuid
from pathlib import Path
from typing import Any, Callable

import numpy as np
from PIL import Image

from manga_reader_ai.config import settings
from manga_reader_ai.models.schemas import SynthesisResult, ScriptSegment

logger = logging.getLogger(__name__)


def _decode_png_b64(b64: str) -> Image.Image:
    """Decode a base64 PNG string to a PIL Image."""
    raw = base64.b64decode(b64)
    return Image.open(io.BytesIO(raw)).convert("RGB")


def _fit_to_background(
    img: Image.Image, background_size: tuple[int, int] = (1280, 720)
) -> np.ndarray:
    """Fit an image onto a black canvas of the given size (letterbox)."""
    bg = Image.new("RGB", background_size, (0, 0, 0))
    bg_w, bg_h = background_size
    img_w, img_h = img.size
    scale = min(bg_w / img_w, bg_h / img_h)
    new_w = int(img_w * scale)
    new_h = int(img_h * scale)
    resized = img.resize((new_w, new_h), Image.Resampling.LANCZOS)
    x = (bg_w - new_w) // 2
    y = (bg_h - new_h) // 2
    bg.paste(resized, (x, y))
    return np.array(bg)


def _estimate_duration_seconds(text: str, speed: float) -> float:
    """Rough estimate of speech duration based on character count."""
    sps = 13.0 * max(0.2, float(speed))
    return max(0.8, len(text.strip()) / max(1.0, sps))


# ── Kokoro TTS Engine ──────────────────────────────────────────────────


class KokoroEngine:
    """Local Kokoro TTS engine.

    Wraps the Kokoro KPipeline for async-friendly generation.
    Ported from video_director_kokoro._synthesize_kokoro_to_wav.
    """

    def __init__(self, voice: str = "af_heart", speed: float = 1.0) -> None:
        self.voice = voice
        self.speed = speed
        self._pipeline: Any = None

    @property
    def pipeline(self) -> Any:
        if self._pipeline is None:
            from kokoro import KPipeline

            self._pipeline = KPipeline(lang_code="a")
        return self._pipeline

    async def synthesize(
        self,
        text: str,
        voice: str | None = None,
        speed: float | None = None,
    ) -> SynthesisResult:
        """Synthesise text to WAV audio bytes.

        Returns:
            SynthesisResult with base64-encoded audio and duration.
        """
        actual_voice = voice or self.voice
        actual_speed = max(0.5, float(speed or self.speed))

        pipe = self.pipeline

        # Run the generator in a thread pool (KPipeline is synchronous)
        def _run() -> tuple[bytes, float, int]:
            gen = pipe(text, voice=actual_voice, speed=actual_speed)
            all_audio: list[np.ndarray] = []
            sr = 24000
            for result in gen:
                if hasattr(result, "audio"):
                    chunk = np.array(
                        result.audio.cpu().numpy() if hasattr(result.audio, "cpu") else result.audio,
                        dtype=np.float32,
                    ).squeeze()
                elif isinstance(result, tuple):
                    chunk = result[0]
                    sr = result[1] if len(result) >= 2 else sr
                    chunk = np.array(chunk, dtype=np.float32).squeeze()
                else:
                    continue
                if chunk.size > 0:
                    all_audio.append(chunk)

            if not all_audio:
                raise RuntimeError("Kokoro returned empty audio (no chunks generated)")

            audio = np.concatenate(all_audio) if len(all_audio) > 1 else all_audio[0]

            # Normalise
            peak = float(np.max(np.abs(audio)))
            if peak > 0:
                audio = (audio / max(peak, 1e-9)) * min(0.9, peak)

            duration = float(len(audio) / float(sr))

            # Convert to WAV bytes
            import soundfile as sf

            wav_buf = io.BytesIO()
            sf.write(wav_buf, audio, int(sr), format="WAV")
            wav_bytes = wav_buf.getvalue()

            return wav_bytes, duration, int(sr)

        wav_bytes, duration, sr = await asyncio.to_thread(_run)

        return SynthesisResult(
            audio_bytes=base64.b64encode(wav_bytes).decode("ascii"),
            duration_seconds=duration,
            sample_rate=sr,
            provider="kokoro",
        )

    async def synthesize_to_file(
        self,
        text: str,
        output_path: str | Path,
        voice: str | None = None,
        speed: float | None = None,
    ) -> SynthesisResult:
        """Synthesise and save WAV to a file."""
        result = await self.synthesize(text, voice=voice, speed=speed)
        wav_bytes = base64.b64decode(result.audio_bytes)
        out = Path(output_path)
        out.parent.mkdir(parents=True, exist_ok=True)
        out.write_bytes(wav_bytes)
        logger.info(f"[kokoro] WAV saved: {out} ({result.duration_seconds:.3f}s)")
        return result


# ── Movie Builder (port from video_director_kokoro.make_movie_kokoro) ──


RESOLUTIONS: dict[str, tuple[int, int]] = {
    "720p": (1280, 720),
    "1080p": (1920, 1080),
}


class MovieBuilder:
    """Builds MP4 movies from script segments with Kokoro TTS audio.

    Ported from video_director_kokoro.make_movie_kokoro with async refactoring.
    """

    def __init__(
        self,
        tts_engine: KokoroEngine | None = None,
        voice: str = "af_heart",
        speed: float = 1.0,
        resolution: str = "720p",
        tmp_dir: str | Path | None = None,
    ) -> None:
        self.tts = tts_engine or KokoroEngine(voice=voice, speed=speed)
        self.voice = voice
        self.speed = speed
        self.resolution = resolution
        self.bg_w, self.bg_h = RESOLUTIONS.get(resolution, RESOLUTIONS["720p"])
        self.tmp_dir = Path(tmp_dir or settings.tmp_dir) if tmp_dir else Path(settings.tmp_dir)
        self._cancel = asyncio.Event()

    def cancel(self) -> None:
        """Signal cancellation."""
        self._cancel.set()

    @property
    def cancelled(self) -> bool:
        return self._cancel.is_set()

    async def build(
        self,
        script: list[ScriptSegment | dict[str, Any]],
        output_path: str | Path,
        progress_callback: Callable[[int, str], None] | None = None,
    ) -> Path:
        """Build an MP4 from script segments.

        Each segment should have:
            text (str): narration text
            images (list[str]): base64-encoded PNG images (scaled, fallback)
            images_unscaled (list[str]): base64-encoded full-res images (preferred)
            page_indices (list[int]): associated page numbers

        Returns path to the output MP4.
        """
        if self.cancelled:
            raise RuntimeError("Movie build cancelled")

        out_path = Path(output_path)
        out_path.parent.mkdir(parents=True, exist_ok=True)
        work_dir = self.tmp_dir / f"movie_{uuid.uuid4().hex}"
        work_dir.mkdir(parents=True, exist_ok=True)

        try:
            video_info = await self._build_segments(script, work_dir, progress_callback)
            if self.cancelled:
                raise RuntimeError("Movie build cancelled during segment processing")
            await self._render_movie(video_info, out_path, work_dir)
        finally:
            self._cleanup_tmp(work_dir)

        logger.info(f"[movie] MP4 saved: {out_path}")
        return out_path

    async def _build_segments(
        self,
        script: list[ScriptSegment | dict[str, Any]],
        work_dir: Path,
        progress_callback: Callable[[int, str], None] | None,
    ) -> list[dict[str, Any]]:
        """Process each script segment: synthesize audio, prepare image clips."""
        from moviepy import AudioFileClip

        video_info: list[dict[str, Any]] = []

        for seg_idx, segment in enumerate(script):
            if self.cancelled:
                raise RuntimeError("Cancelled")

            if isinstance(segment, dict):
                seg = ScriptSegment(**segment)
            else:
                seg = segment

            text = seg.text
            images_b64 = seg.images_unscaled or seg.images or []

            if not images_b64 or not text.strip():
                continue

            if progress_callback:
                pct = 60 + int(30 * seg_idx / max(1, len(script)))
                progress_callback(pct, f"Synthesising audio {seg_idx + 1}/{len(script)}...")

            # Synthesise audio
            tts_result = await self.tts.synthesize(
                text=text, voice=self.voice, speed=self.speed
            )

            wav_path = work_dir / f"seg_{seg_idx:04d}.wav"
            wav_bytes = base64.b64decode(tts_result.audio_bytes)
            wav_path.write_bytes(wav_bytes)

            audio_clip = AudioFileClip(str(wav_path))
            audio_dur = float(getattr(audio_clip, "duration", 0.0) or 0.0)

            if audio_dur <= 0.001:
                raise RuntimeError(
                    f"TTS produced near-zero duration ({audio_dur:.6f}s) for segment {seg_idx}"
                )

            image_dur = audio_dur / max(1, len(images_b64))
            image_clips: list[Any] = []

            for img_b64 in images_b64:
                if self.cancelled:
                    raise RuntimeError("Cancelled")
                img = _decode_png_b64(img_b64)
                frame = _fit_to_background(img, (self.bg_w, self.bg_h))
                from moviepy import ImageClip

                image_clips.append(ImageClip(frame).with_duration(image_dur))

            from moviepy import concatenate_videoclips

            seg_video = concatenate_videoclips(image_clips, method="compose")

            video_info.append(
                {
                    "video": seg_video,
                    "audio": audio_clip,
                    "wav_path": wav_path,
                }
            )

        if not video_info:
            raise RuntimeError("No content generated — script was empty")

        return video_info

    async def _render_movie(
        self,
        video_info: list[dict[str, Any]],
        out_path: Path,
        work_dir: Path,
    ) -> None:
        """Concatenate clips and write the final MP4."""
        from moviepy import (
            AudioClip,
            VideoClip,
            concatenate_audioclips,
            concatenate_videoclips,
        )

        video_clips: list[VideoClip] = [v["video"] for v in video_info]
        audio_clips: list[AudioClip] = [v["audio"] for v in video_info]

        final_video = concatenate_videoclips(video_clips, method="compose")
        final_audio = concatenate_audioclips(audio_clips)
        final_video = final_video.with_audio(final_audio)

        tmp_output = work_dir / "output_tmp.mp4"

        def _write() -> None:
            final_video.write_videofile(
                str(tmp_output),
                codec="libx264",
                audio_codec="aac",
                fps=24,
                preset="veryfast",
                bitrate="2000k",
                ffmpeg_params=["-crf", "28", "-loglevel", "error"],
                temp_audiofile_path=str(work_dir),
                remove_temp=True,
                logger=None,
            )

        await asyncio.to_thread(_write)

        # Move to final location
        import shutil

        if out_path.exists():
            out_path.unlink()
        shutil.move(str(tmp_output), str(out_path))

        # Close clips
        for clip in audio_clips:
            try:
                clip.close()
            except Exception:
                pass
        for clip in video_clips:
            try:
                clip.close()
            except Exception:
                pass
        try:
            final_video.close()
        except Exception:
            pass
        try:
            final_audio.close()
        except Exception:
            pass

    @staticmethod
    def _cleanup_tmp(work_dir: Path) -> None:
        """Remove temporary working directory."""
        import shutil

        try:
            if work_dir.exists():
                shutil.rmtree(work_dir)
        except Exception:
            pass
