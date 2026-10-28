from pathlib import Path
from PIL import Image, ImageDraw
import wave, subprocess

fixtures_dir = Path("models/_test_fixtures")
fixtures_dir.mkdir(parents=True, exist_ok=True)

# 1. OCR Single-line text
img_single = Image.new("RGB", (320, 48), color=(255, 255, 255))
d = ImageDraw.Draw(img_single)
d.text((10, 14), "STOP CALLING ME", fill=(0, 0, 0))
img_single.save(fixtures_dir / "ocr_single_line.png")
print("Created ocr_single_line.png")

# 2. OCR Multi-line document
img_multi = Image.new("RGB", (320, 200), color=(255, 255, 255))
d_m = ImageDraw.Draw(img_multi)
d_m.text((10, 20), "INCIDENT REPORT", fill=(0, 0, 0))
d_m.text((10, 60), "STOP HARASSING ME", fill=(0, 0, 0))
d_m.text((10, 100), "LEAVE ME ALONE IMMEDIATELY", fill=(0, 0, 0))
img_multi.save(fixtures_dir / "ocr_multiline.png")
print("Created ocr_multiline.png")

# 3. Audio Speech WAV via Windows SAPI
out_speech = (fixtures_dir / "speech_stop_calling_me.wav").resolve()
ps_cmd = f'$s = New-Object -ComObject SAPI.SpVoice; $fs = New-Object -ComObject SAPI.SpFileStream; $fs.Open("{str(out_speech).replace(chr(92), chr(47))}", 3); $s.AudioOutputStream = $fs; $s.Speak("Stop calling me."); $fs.Close()'
subprocess.run(["powershell", "-NoProfile", "-Command", ps_cmd], check=True)
print("Created speech_stop_calling_me.wav")

# 4. Audio Silence WAV
out_silence = fixtures_dir / "silence.wav"
with wave.open(str(out_silence), "wb") as w:
    w.setnchannels(1)
    w.setsampwidth(2)
    w.setframerate(16000)
    w.writeframes(b"\x00\x00" * 32000)
print("Created silence.wav")

# 5. Invalid Audio
with open(fixtures_dir / "corrupt.wav", "wb") as f:
    f.write(b"RIFF\x10\x00\x00\x00WAVEbadheader")
print("Created corrupt.wav")

