#!/bin/bash
# audio.sh <l|v>: the mix. edm-5 (140 bpm, G harmonic minor) from 12.857 s, its beat 30, so the drop (beat 32) lands
# at 0.857 s and every beat sits on k·(60/140) s from frame 0. A soft swish peaks on each section join's beat and a
# faint G-minor chime (G5, D6, G6) sits under the signed reveal and the Jlorious back. Writes mix-<l|v>.wav.
set -e
cd "$(dirname "$0")"
FF=$(cat ffpath.txt)
MUSIC=../../apps/web/public/audio/music/edm-5.m4a
SR=48000
B=$(python3 -c "print(60/140)")
cut=$1
if [ "$cut" = v ]; then DUR=13.7; SW="13 19 28"; CH="6 19"; FADE="afade=t=in:st=0:d=0.01,afade=t=out:st=13.55:d=0.15";
else DUR=20; SW="16 24 32 42"; CH="6 32"; FADE="afade=t=out:st=19:d=1"; fi
"$FF" -y -loglevel error -f lavfi -i "aevalsrc='(random(0)*2-1)*if(lt(t,0.12),pow(t/0.12,2),exp(-(t-0.12)*13))*(1-0.6*t/0.42)|(random(1)*2-1)*if(lt(t,0.12),pow(t/0.12,2),exp(-(t-0.12)*13))*(0.4+0.6*t/0.42)':s=$SR:d=0.42" \
  -af "highpass=f=700,lowpass=f=5200,aecho=0.7:0.35:38|71:0.22|0.12" swish.wav
"$FF" -y -loglevel error -f lavfi -i "aevalsrc='(0.55*sin(2*PI*783.99*t)+0.35*sin(2*PI*1174.66*t)+0.25*sin(2*PI*1567.98*t))*min(1,t/0.01)*exp(-t*2.6)|(0.5*sin(2*PI*783.99*t)+0.38*sin(2*PI*1174.66*t)+0.28*sin(2*PI*1567.98*t))*min(1,t/0.01)*exp(-t*2.6)':s=$SR:d=1.6" chime.wav
SW_VOL=${SW_VOL:-2.6}; CH_VOL=${CH_VOL:-0.08}
ins=(-ss 12.857142857 -i "$MUSIC"); fc="[0:a]atrim=0:$DUR,asetpts=PTS-STARTPTS,aresample=$SR,aformat=sample_fmts=fltp:channel_layouts=stereo,$FADE[m];"; mix="[m]"; n=1
for k in $SW; do ms=$(python3 -c "print(round(($k*$B-0.12)*1000))"); ins+=(-i swish.wav); fc="$fc[$n:a]volume=$SW_VOL,adelay=$ms|$ms,aformat=sample_fmts=fltp:channel_layouts=stereo[a$n];"; mix="$mix[a$n]"; n=$((n+1)); done
for k in $CH; do ms=$(python3 -c "print(round($k*$B*1000))"); ins+=(-i chime.wav); fc="$fc[$n:a]volume=$CH_VOL,adelay=$ms|$ms,aformat=sample_fmts=fltp:channel_layouts=stereo[a$n];"; mix="$mix[a$n]"; n=$((n+1)); done
fc="$fc${mix}amix=inputs=$n:normalize=0:duration=first,alimiter=limit=0.89:level=false,apad=whole_dur=$DUR,atrim=0:$DUR[out]"
"$FF" -y -loglevel error "${ins[@]}" -filter_complex "$fc" -map "[out]" -ar $SR -c:a pcm_s16le "mix-$cut.wav"
echo "mix-$cut.wav $("$FF" -hide_banner -i "mix-$cut.wav" 2>&1 | grep -o 'Duration: [0-9:.]*')"
