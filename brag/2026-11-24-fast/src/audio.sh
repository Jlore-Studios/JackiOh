#!/bin/bash
# audio.sh land|vert: edm-2 (172 bpm, F minor) from its drums' entry at beat 32 (11.163 s), so every beat lands on
# the video's beat grid from frame 0; a low F impact under the lethal hit (beat 12) and, in land, a soft swish
# peaking on the outro's beat (50). Writes mix-<fmt>.wav, 48 kHz stereo, exactly the video's length.
set -e
cd "$(dirname "$0")"
FF=$(cat ffpath.txt)
MUSIC=../../apps/web/public/audio/music/edm-2.m4a
SR=48000
BEAT=$(python3 -c "print(60/172)")
OFF=$(python3 -c "print(32*60/172)")
fmt=${1:-land}
if [ "$fmt" = land ]; then DUR=$(python3 -c "print(628/30)"); FADE_ST=20.25; FADE_D=0.68; else DUR=$(python3 -c "print(419/30)"); FADE_ST=$(python3 -c "print(419/30-0.12)"); FADE_D=0.12; fi
HIT=$(python3 -c "print(round(12*60/172*1000))")
SWP=$(python3 -c "print(round(50*60/172*1000)-120)")
# The impact: F1 and F2 sines falling a few hertz, a fast attack and a short decay, with a soft low noise thump.
"$FF" -y -loglevel error -f lavfi -i "aevalsrc='(0.8*sin(2*PI*(43.65*t-6*t*t))+0.45*sin(2*PI*(87.31*t-10*t*t))+0.25*(random(0)*2-1)*exp(-t*40))*min(1,t/0.004)*exp(-t*4.2)':s=$SR:d=0.9" -af "lowpass=f=900" boom.wav
# The swish: band-limited noise swelling for 0.12 s to its peak, then falling away, panned left to right.
"$FF" -y -loglevel error -f lavfi -i "aevalsrc='(random(0)*2-1)*if(lt(t,0.12),pow(t/0.12,2),exp(-(t-0.12)*13))*(1-0.6*t/0.42)|(random(1)*2-1)*if(lt(t,0.12),pow(t/0.12,2),exp(-(t-0.12)*13))*(0.4+0.6*t/0.42)':s=$SR:d=0.42" -af "highpass=f=700,lowpass=f=5200,aecho=0.7:0.35:38|71:0.22|0.12" swish.wav
BOOM_VOL=${BOOM_VOL:-0.35}
SW_VOL=${SW_VOL:-2.2}
"$FF" -y -loglevel error -ss "$OFF" -i "$MUSIC" -i boom.wav -i swish.wav -filter_complex "
  [0:a]atrim=0:$DUR,asetpts=PTS-STARTPTS,aresample=$SR,aformat=sample_fmts=fltp:channel_layouts=stereo,afade=t=in:st=0:d=0.01,afade=t=out:st=$FADE_ST:d=$FADE_D[m];
  [1:a]volume=$BOOM_VOL,adelay=$HIT|$HIT,aformat=sample_fmts=fltp:channel_layouts=stereo[b];
  [2:a]volume=$( [ "$fmt" = land ] && echo $SW_VOL || echo 0 ),adelay=$SWP|$SWP,aformat=sample_fmts=fltp:channel_layouts=stereo[s];
  [m][b][s]amix=inputs=3:normalize=0:duration=first,alimiter=limit=0.89:level=false,apad=whole_dur=$DUR,atrim=0:$DUR[out]" \
  -map "[out]" -ar $SR -c:a pcm_s16le "mix-$fmt.wav"
echo "mix-$fmt.wav $DUR s"
