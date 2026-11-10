#!/bin/sh
set -eu
cd "$(dirname "$0")/web/fixtures"
ffmpeg -hide_banner -loglevel error -y -f lavfi -i 'sine=frequency=440:sample_rate=48000' \
  -t 240 -c:a libmp3lame -b:a 128k continuous.mp3
ffmpeg -hide_banner -loglevel error -y \
  -f lavfi -i 'testsrc2=size=1920x1080:rate=30' \
  -f lavfi -i 'sine=frequency=660:sample_rate=48000' \
  -t 12 -c:v libx264 -preset fast -crf 23 -pix_fmt yuv420p -c:a aac \
  -profile:a aac_low -movflags +faststart candidate.mp4
