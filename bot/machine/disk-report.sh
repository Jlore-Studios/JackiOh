#!/bin/bash
# What fills the machine's disk, biggest first. Read-only; run it from your computer:
#
#   bot/machine/on-machine.sh bot/machine/disk-report.sh
#
# The issue "Night bot: the machine's disk is filling up" (bot/harness/disk.py) points here.
set +e
df -h /
echo
echo "Biggest under each agent's home, /tmp, /var and the system:"
du -xsh /home/agent-*/* /home/agent-*/.[!.]* /home/agent-*/actions-runner*/* /tmp /var/log \
  /var/lib /var/cache /usr /opt /snap /swapfile /root 2>/dev/null | sort -rh | head -40
echo
echo "Runner versions kept beside the current one (clean.sh removes those over an hour old):"
ls -d /home/agent-*/actions-runner*/bin.* /home/agent-*/actions-runner*/externals.* 2>/dev/null
echo
echo "The disk timer's last runs:"
journalctl -t night-vm-disk -n 20 --no-pager 2>/dev/null
