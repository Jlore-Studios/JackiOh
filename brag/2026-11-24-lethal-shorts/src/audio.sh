#!/bin/bash
# audio.sh <cut>: the mix for one cut, mix-<cut>.wav (48 kHz stereo, 14.000 s). edm-1 from 0 s (D minor, 120 bpm,
# its kick every 0.5 s from 0), fading over its last 0.4 s so the loop restarts clean; a soft swish peaking on each
# flip's beat (2.0 s, the takeover; 9.0 s, the Radiant flip), a faint D-minor chime (D5, A5, D6) under the second,
# and in cut c a reversed swish under the rewind (1.0-1.5 s).
set -e
cd "$(dirname "$0")"
CUT=${1:-a}
FF=$(cat ffpath.txt)
MUSIC=../../apps/web/public/audio/music/edm-1.m4a
SR=48000; D=14
"$FF" -y -loglevel error -f lavfi -i "aevalsrc='(random(0)*2-1)*if(lt(t,0.12),pow(t/0.12,2),exp(-(t-0.12)*13))*(1-0.6*t/0.42)|(random(1)*2-1)*if(lt(t,0.12),pow(t/0.12,2),exp(-(t-0.12)*13))*(0.4+0.6*t/0.42)':s=$SR:d=0.42" \
  -af "highpass=f=700,lowpass=f=5200,aecho=0.7:0.35:38|71:0.22|0.12" swish.wav
"$FF" -y -loglevel error -f lavfi -i "aevalsrc='(0.55*sin(2*PI*587.33*t)+0.35*sin(2*PI*880*t)+0.25*sin(2*PI*1174.66*t))*min(1,t/0.01)*exp(-t*2.6)|(0.5*sin(2*PI*587.33*t)+0.38*sin(2*PI*880*t)+0.28*sin(2*PI*1174.66*t))*min(1,t/0.01)*exp(-t*2.6)':s=$SR:d=1.6" chime.wav
SW_VOL=${SW_VOL:-2.6}; CH_VOL=${CH_VOL:-0.08}; RW_VOL=${RW_VOL:-2.6}
REW_IN=""; REW_F=""; REW_L=""
if [ "$CUT" = c ]; then
  REW_IN="-i swish.wav"
  REW_F="[4:a]areverse,atempo=0.85,volume=$RW_VOL,adelay=1000|1000,aformat=sample_fmts=fltp:channel_layouts=stereo[r];"
  REW_L="[r]"
fi
N=$([ "$CUT" = c ] && echo 5 || echo 4)
"$FF" -y -loglevel error -i "$MUSIC" -i swish.wav -i swish.wav -i chime.wav $REW_IN -filter_complex "
  [0:a]atrim=0:$D,asetpts=PTS-STARTPTS,aresample=$SR,aformat=sample_fmts=fltp:channel_layouts=stereo,afade=t=out:st=13.6:d=0.4[m];
  [1:a]volume=$SW_VOL,adelay=1880|1880,aformat=sample_fmts=fltp:channel_layouts=stereo[s1];
  [2:a]volume=$SW_VOL,adelay=8880|8880,aformat=sample_fmts=fltp:channel_layouts=stereo[s2];
  [3:a]volume=$CH_VOL,adelay=9000|9000,aformat=sample_fmts=fltp:channel_layouts=stereo[c];
  $REW_F
  [m][s1][s2][c]${REW_L}amix=inputs=$N:normalize=0:duration=first,alimiter=limit=0.89:level=false,apad=whole_dur=$D,atrim=0:$D[out]" \
  -map "[out]" -ar $SR -c:a pcm_s16le "mix-$CUT.wav"
echo "mix-$CUT.wav written"
