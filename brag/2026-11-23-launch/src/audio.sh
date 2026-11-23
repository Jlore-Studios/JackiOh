#!/bin/bash
# The mix: edm-1 0–20 s fading from 19 s, a soft swish peaking on each flip's beat (2.0 s, 13.0 s) and a
# faint D-minor chime (D5, A5, D6) under the Radiant flip. Writes mix.wav (48 kHz stereo, 20.000 s).
set -e
cd "$(dirname "$0")"
FF=$(cat ffpath.txt)
MUSIC=../../apps/web/public/audio/music/edm-1.m4a
SR=48000
# The swish: band-limited noise, swelling for 0.12 s to its peak and falling away, panned left to right.
"$FF" -y -loglevel error -f lavfi -i "aevalsrc='(random(0)*2-1)*if(lt(t,0.12),pow(t/0.12,2),exp(-(t-0.12)*13))*(1-0.6*t/0.42)|(random(1)*2-1)*if(lt(t,0.12),pow(t/0.12,2),exp(-(t-0.12)*13))*(0.4+0.6*t/0.42)':s=$SR:d=0.42" \
  -af "highpass=f=700,lowpass=f=5200,aecho=0.7:0.35:38|71:0.22|0.12" swish.wav
# The chime: three sines of the track's key, a soft bell decay.
"$FF" -y -loglevel error -f lavfi -i "aevalsrc='(0.55*sin(2*PI*587.33*t)+0.35*sin(2*PI*880*t)+0.25*sin(2*PI*1174.66*t))*min(1,t/0.01)*exp(-t*2.6)|(0.5*sin(2*PI*587.33*t)+0.38*sin(2*PI*880*t)+0.28*sin(2*PI*1174.66*t))*min(1,t/0.01)*exp(-t*2.6)':s=$SR:d=1.6" chime.wav
SW_VOL=${SW_VOL:-2.6}
CH_VOL=${CH_VOL:-0.08}
"$FF" -y -loglevel error -i "$MUSIC" -i swish.wav -i swish.wav -i chime.wav -filter_complex "
  [0:a]atrim=0:20,asetpts=PTS-STARTPTS,aresample=$SR,aformat=sample_fmts=fltp:channel_layouts=stereo,afade=t=out:st=19:d=1[m];
  [1:a]volume=$SW_VOL,adelay=1880|1880,aformat=sample_fmts=fltp:channel_layouts=stereo[s1];
  [2:a]volume=$SW_VOL,adelay=12880|12880,aformat=sample_fmts=fltp:channel_layouts=stereo[s2];
  [3:a]volume=$CH_VOL,adelay=13000|13000,aformat=sample_fmts=fltp:channel_layouts=stereo[c];
  [m][s1][s2][c]amix=inputs=4:normalize=0:duration=first,alimiter=limit=0.89:level=false,apad=whole_dur=20,atrim=0:20[out]" \
  -map "[out]" -ar $SR -c:a pcm_s16le mix.wav
echo "mix.wav written"
