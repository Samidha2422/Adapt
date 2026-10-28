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

# 2. OCR Multi-line document (3 lines)
img_multi = Image.new("RGB", (320, 200), color=(255, 255, 255))
d_m = ImageDraw.Draw(img_multi)
d_m.text((10, 20), "INCIDENT REPORT", fill=(0, 0, 0))
d_m.text((10, 60), "STOP HARASSING ME", fill=(0, 0, 0))
d_m.text((10, 100), "LEAVE ME ALONE IMMEDIATELY", fill=(0, 0, 0))
img_multi.save(fixtures_dir / "ocr_multiline.png")
print("Created ocr_multiline.png")

# 2b. OCR Longer Multi-line Page (5 lines)
img_long = Image.new("RGB", (480, 360), color=(255, 255, 255))
d_l = ImageDraw.Draw(img_long)
d_l.text((15, 20), "FORMAL WARNING NOTICE", fill=(0, 0, 0))
d_l.text((15, 70), "PERSISTENT HARASSMENT LOGGED", fill=(0, 0, 0))
d_l.text((15, 120), "DO NOT CONTACT THIS NUMBER AGAIN", fill=(0, 0, 0))
d_l.text((15, 170), "FURTHER ATTEMPTS WILL BE REPORTED", fill=(0, 0, 0))
d_l.text((15, 220), "CASE REFERENCE REF94827", fill=(0, 0, 0))
img_long.save(fixtures_dir / "ocr_long_multiline.png")
print("Created ocr_long_multiline.png")

# 2c. OCR No Text Image
img_no_text = Image.new("RGB", (320, 240), color=(240, 245, 250))
img_no_text.save(fixtures_dir / "ocr_no_text.png")
print("Created ocr_no_text.png")

# 2d. OCR Varied Widths
img_varied = Image.new("RGB", (400, 240), color=(255, 255, 255))
d_v = ImageDraw.Draw(img_varied)
d_v.text((10, 20), "ALERT", fill=(0, 0, 0))
d_v.text((10, 70), "VIOLATION DETECTED", fill=(0, 0, 0))
d_v.text((10, 120), "WE HAVE LOGGED YOUR PERSISTENT ABUSE ACROSS CHANNELS", fill=(0, 0, 0))
img_varied.save(fixtures_dir / "ocr_varied_widths.png")
print("Created ocr_varied_widths.png")

# 2e. OCR Large Boundary Image (1920x1080)
img_large = Image.new("RGB", (1920, 1080), color=(250, 250, 250))
d_large = ImageDraw.Draw(img_large)
d_large.text((50, 100), "HIGH RESOLUTION EVIDENCE SCREENSHOT", fill=(0, 0, 0))
d_large.text((50, 300), "UNSOLICITED THREATENING MESSAGES RECORDED", fill=(0, 0, 0))
img_large.save(fixtures_dir / "ocr_large_boundary.png")
print("Created ocr_large_boundary.png")

# 2f. Corrupt Image
with open(fixtures_dir / "ocr_corrupt.png", "wb") as f:
    f.write(b"NOT_A_VALID_PNG_FILE_HEADER")
print("Created ocr_corrupt.png")

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
