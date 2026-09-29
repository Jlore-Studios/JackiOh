"""Time: the current instant, ISO formatting, and the nightly run window."""

from __future__ import annotations

from dataclasses import dataclass
from datetime import datetime, time, timedelta, timezone, tzinfo

try:
    from zoneinfo import ZoneInfo, ZoneInfoNotFoundError
except ImportError:  # pragma: no cover - every supported Python has zoneinfo
    ZoneInfo = None  # type: ignore[assignment]
    ZoneInfoNotFoundError = Exception  # type: ignore[assignment,misc]


def iso(dt: datetime) -> str:
    """`dt` as ISO-8601 UTC with a trailing Z, to the second."""
    return dt.astimezone(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def parse_iso(text: str | None) -> datetime | None:
    """An aware UTC datetime from ISO text, or None when `text` is empty or unreadable."""
    if not text:
        return None
    try:
        parsed = datetime.fromisoformat(str(text).strip().replace("Z", "+00:00"))
    except ValueError:
        return None
    if parsed.tzinfo is None:
        parsed = parsed.replace(tzinfo=timezone.utc)
    return parsed.astimezone(timezone.utc)


def now(override: str = "") -> datetime:
    """The current instant, or `override` (HARNESS_NOW) when it parses."""
    fixed = parse_iso(override)
    return fixed if fixed is not None else datetime.now(timezone.utc)


def zone(name: str) -> tzinfo:
    """The named IANA zone. America/Chicago falls back to fixed US rules without tzdata."""
    if ZoneInfo is not None:
        try:
            return ZoneInfo(name)
        except ZoneInfoNotFoundError:
            pass
    if name == "America/Chicago":
        return _UsCentral()
    return timezone.utc


class _UsCentral(tzinfo):
    """US Central time with the 2007 DST rules, for hosts without tzdata."""

    def _dst_bounds(self, year: int) -> tuple[datetime, datetime]:
        march = datetime(year, 3, 8)
        start = march + timedelta(days=(6 - march.weekday()) % 7)
        november = datetime(year, 11, 1)
        end = november + timedelta(days=(6 - november.weekday()) % 7)
        return start.replace(hour=2), end.replace(hour=2)

    def utcoffset(self, dt: datetime | None) -> timedelta:
        return timedelta(hours=-6) + self.dst(dt)

    def dst(self, dt: datetime | None) -> timedelta:
        if dt is None:
            return timedelta(0)
        start, end = self._dst_bounds(dt.year)
        naive = dt.replace(tzinfo=None)
        return timedelta(hours=1) if start <= naive < end else timedelta(0)

    def tzname(self, dt: datetime | None) -> str:
        return "CDT" if self.dst(dt) else "CST"

    def fromutc(self, dt: datetime) -> datetime:
        standard = dt + timedelta(hours=-6)
        start, end = self._dst_bounds(standard.year)
        naive = standard.replace(tzinfo=None)
        # DST starts at 02:00 standard time and ends at 02:00 daylight time (01:00 standard).
        if start <= naive < end - timedelta(hours=1):
            return standard + timedelta(hours=1)
        return standard


def parse_hhmm(text: str) -> time:
    hours, minutes = str(text).strip().split(":")
    return time(int(hours), int(minutes))


@dataclass(frozen=True)
class Window:
    """A daily window in a local zone, such as 21:00 to 07:00 America/Chicago."""

    zone_name: str
    start: time
    end: time

    @classmethod
    def of(cls, zone_name: str, start: str, end: str) -> "Window":
        return cls(zone_name, parse_hhmm(start), parse_hhmm(end))

    def is_open(self, at: datetime) -> bool:
        local = at.astimezone(zone(self.zone_name)).time().replace(tzinfo=None)
        if self.start <= self.end:
            return self.start <= local < self.end
        return local >= self.start or local < self.end

    def _local_instant(self, day: datetime, at: time) -> datetime:
        tz = zone(self.zone_name)
        return datetime(day.year, day.month, day.day, at.hour, at.minute, tzinfo=tz)

    def next_open(self, at: datetime) -> datetime:
        """The next instant the window opens, at or after `at` (UTC)."""
        local = at.astimezone(zone(self.zone_name))
        for offset in range(0, 3):
            day = local + timedelta(days=offset)
            candidate = self._local_instant(day, self.start).astimezone(timezone.utc)
            if candidate >= at:
                return candidate
        raise AssertionError("a daily window opens within three days")

    def closes_at(self, at: datetime) -> datetime | None:
        """When the window that is open at `at` closes, or None when it is closed."""
        if not self.is_open(at):
            return None
        local = at.astimezone(zone(self.zone_name))
        for offset in range(0, 3):
            day = local + timedelta(days=offset)
            candidate = self._local_instant(day, self.end).astimezone(timezone.utc)
            if candidate > at:
                return candidate
        raise AssertionError("a daily window closes within three days")

    def describe(self) -> str:
        return f"{self.start:%H:%M}–{self.end:%H:%M} {self.zone_name}"


def human_delta(delta: timedelta) -> str:
    """`3h 05m`, `12m` or `now`."""
    minutes = int(delta.total_seconds() // 60)
    if minutes <= 0:
        return "now"
    hours, minutes = divmod(minutes, 60)
    return f"{hours}h {minutes:02d}m" if hours else f"{minutes}m"
