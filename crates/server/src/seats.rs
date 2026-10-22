//! The seat ledger (SPEC §9.5, §13.9; R1523000, which amends R890000): the one game each profile
//! holds, a live 1v1 match, a Conquest series that is not over or a live co-op match, kept in this
//! process so that every door into a game asks one question and takes its seats in the same step.
//!
//! A door reserves all of its seats at once (`Seats::reserve`): every profile it names, or none. A
//! reserved seat is pending until the door's game stands (`Lease::commit`), and a pending seat
//! already counts as taken, so a rival door is refused at once; a lease dropped uncommitted puts
//! back what each profile held before it. A held seat admits only itself again (an idempotent
//! retry, and R639000's bot that its rematch claim seated first) and, for a series between games,
//! that series' next game (R264). Anything else is `Taken`, which the doors answer with R1065000's
//! refusals. Bots are profiles and reserve like people.
//!
//! The in-match flag (`profiles.current_match_id`) stays the record and a second guard: the doors
//! still write it compare-and-set (R890000) and the endings still clear it. Boot fills the ledger
//! from the record (`boot`), and every reaper tick compares the two (`reconcile`): a seat whose
//! game ended without its release is freed, and one the record holds that the ledger lacks is
//! adopted, each logged as `seats.drift`.
//!
//! Two processes overlap while a deploy hands over, each with its own ledger, so every claim fences
//! on `app.seat_epoch` (migration 0054) inside the transaction that writes its seats: boot bumps the
//! epoch before it fills the ledger, and a claim made on an older epoch writes nothing
//! (`StoreError::Fenced`, answered 503). The fence's shared row lock makes the bump wait for the
//! claims in flight, so the new ledger reads every seat the old process committed. No lock is held
//! in the database past a transaction.
//!
//! The mutex is held only for an in-memory check and write, never across an `.await`.

use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Mutex, MutexGuard};

use indexmap::{IndexMap, IndexSet};

use crate::actor::contracts::one_tx;
use crate::app::App;
use crate::db::store::{SeatedProfile, SeriesRow, SeriesStatus, StoreError};

/// The game a profile holds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Seat {
    /// A live 1v1 match, by id: the queue's, a room's or a rematch.
    Match(String),
    /// A Conquest series that is not over, by id (R264), held between its games too; `game` is the
    /// match of the game being played, if one is.
    Series { id: String, game: Option<String> },
    /// A live co-op match, by id (R1065000).
    Duos(String),
}

/// A reservation refused: the first profile asked for whose seat does not admit the new one, and
/// that seat.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Taken {
    pub profile: String,
    pub seat: Seat,
}

/// One profile whose seat `Seats::settle` moved to the record's.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Drift {
    pub profile: String,
    pub ledger: Option<Seat>,
    pub record: Option<Seat>,
}

/// A series that is not over, as the record holds it: its two sides and its games' match ids.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SeriesSeats {
    pub id: String,
    pub sides: [String; 2],
    pub games: Vec<String>,
}

/// A live co-op match, as the record holds it: its two people.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DuosSeats {
    pub id: String,
    pub people: [String; 2],
}

/// The ledger, one per `App` (`App::seats`), and the epoch its claims fence on.
#[derive(Default)]
pub struct Seats {
    inner: Mutex<Inner>,
    epoch: AtomicI64,
}

#[derive(Default)]
struct Inner {
    held: IndexMap<String, Held>,
    /// Counts every change to a profile's seat, so `settle` can leave alone what moved after the
    /// record it compares was read.
    clock: u64,
    /// The profile -> the `clock` of its last change. A stamp is kept while a read in flight
    /// marked before it, so that read's `settle` still leaves that profile alone.
    changed: IndexMap<String, u64>,
    /// The `clock` each read of the record in flight marked (`Seats::mark`), one entry per read:
    /// reconciles overlap (the reaper's tick and the matchmaker's stale-start release), and one
    /// settling must not drop the stamps an older one still needs.
    reading: Vec<u64>,
    /// The last lease's id.
    leases: u64,
}

/// A profile's seat; `lease` is the reservation still pending on it, `None` once committed.
#[derive(Clone, Debug)]
struct Held {
    seat: Seat,
    lease: Option<u64>,
}

impl Inner {
    fn touch(&mut self, profile: &str) {
        self.clock += 1;
        self.changed.insert(profile.to_string(), self.clock);
    }
}

/// Whether a held seat lets its profile take `seat`: never while it is pending; the same seat again
/// (a retry); and a series between games its own next game.
fn admits(held: &Held, seat: &Seat) -> bool {
    if held.lease.is_some() {
        return false;
    }
    if held.seat == *seat {
        return true;
    }
    matches!(
        (&held.seat, seat),
        (Seat::Series { id: held_id, game: None }, Seat::Series { id, game: Some(_) }) if held_id == id
    )
}

/// Whether `held` is the game `seat` names: a match or a co-op match by id, a series by id whatever
/// game it is on.
fn same_game(held: &Seat, seat: &Seat) -> bool {
    match (held, seat) {
        (Seat::Match(a), Seat::Match(b)) | (Seat::Duos(a), Seat::Duos(b)) => a == b,
        (Seat::Series { id: a, .. }, Seat::Series { id: b, .. }) => a == b,
        _ => false,
    }
}

impl Seats {
    fn lock(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// All of `profiles`, or none (R1523000). A profile is reserved, pending, when it holds no seat
    /// or a seat that admits `seat`; the first one in the order given that holds anything else
    /// answers `Taken`, and nothing changes.
    pub fn reserve(&self, profiles: &[&str], seat: Seat) -> Result<Lease<'_>, Taken> {
        let mut inner = self.lock();
        for profile in profiles {
            if let Some(held) = inner.held.get(*profile)
                && !admits(held, &seat)
            {
                return Err(Taken {
                    profile: (*profile).to_string(),
                    seat: held.seat.clone(),
                });
            }
        }
        inner.leases += 1;
        let id = inner.leases;
        let mut prior: Vec<(String, Option<Seat>)> = Vec::with_capacity(profiles.len());
        for profile in profiles {
            if prior.iter().any(|(reserved, _)| reserved == profile) {
                continue;
            }
            let before = inner.held.insert(
                (*profile).to_string(),
                Held {
                    seat: seat.clone(),
                    lease: Some(id),
                },
            );
            prior.push(((*profile).to_string(), before.map(|held| held.seat)));
            inner.touch(profile);
        }
        Ok(Lease {
            seats: self,
            id,
            prior,
            done: false,
        })
    }

    /// The seat the profile holds, a pending one included.
    pub fn of(&self, profile: &str) -> Option<Seat> {
        self.lock().held.get(profile).map(|held| held.seat.clone())
    }

    /// Frees the profile's seat, only while it holds the game `seat` names and holds it committed
    /// (as R890000's clear is conditional): a rival's seat taken meanwhile is left alone.
    pub fn release(&self, profile: &str, seat: &Seat) {
        let mut inner = self.lock();
        if inner
            .held
            .get(profile)
            .is_some_and(|held| held.lease.is_none() && same_game(&held.seat, seat))
        {
            inner.held.swap_remove(profile);
        }
        inner.touch(profile);
    }

    /// The match `match_id` has ended for the profile: a 1v1 seat in it is freed, and a series on
    /// it goes back to between games, which admits the series' next game.
    pub fn end_game(&self, profile: &str, match_id: &str) {
        let mut inner = self.lock();
        let between = match inner.held.get(profile) {
            Some(Held {
                lease: None,
                seat: Seat::Match(id),
            }) if id == match_id => Some(None),
            Some(Held {
                lease: None,
                seat: Seat::Series { id, game: Some(game) },
            }) if game == match_id => Some(Some(Seat::Series {
                id: id.clone(),
                game: None,
            })),
            _ => None,
        };
        match between {
            Some(Some(seat)) => {
                inner.held.insert(profile.to_string(), Held { seat, lease: None });
            }
            Some(None) => {
                inner.held.swap_remove(profile);
            }
            None => {}
        }
        inner.touch(profile);
    }

    /// The epoch this process's claims fence on (`app.seat_epoch`), 0 before `boot`.
    pub fn epoch(&self) -> i64 {
        self.epoch.load(Ordering::SeqCst)
    }

    pub fn set_epoch(&self, epoch: i64) {
        self.epoch.store(epoch, Ordering::SeqCst);
    }

    /// Opens a read of the record at the ledger's clock now, before the read starts: its
    /// `Reading::settle` leaves alone every profile changed after it.
    pub fn mark(&self) -> Reading<'_> {
        let mut inner = self.lock();
        let since = inner.clock;
        inner.reading.push(since);
        Reading { seats: self, since }
    }

    /// The record's seats in place of every committed one; a pending seat stays as it is. Every
    /// profile it moves counts as changed, so a reconcile whose read began before it leaves them be.
    pub fn fill(&self, record: IndexMap<String, Seat>) {
        let mut inner = self.lock();
        let dropped: Vec<String> = inner
            .held
            .iter()
            .filter(|(_, held)| held.lease.is_none())
            .map(|(profile, _)| profile.clone())
            .collect();
        inner.held.retain(|_, held| held.lease.is_some());
        for profile in &dropped {
            inner.touch(profile);
        }
        for (profile, seat) in record {
            if !inner.held.contains_key(&profile) {
                inner.touch(&profile);
                inner.held.insert(profile, Held { seat, lease: None });
            }
        }
    }

    /// Moves every committed seat that differs from the record to the record's and answers each
    /// move. A profile whose seat is pending, or changed after `since` (so after the record was
    /// read), is left alone.
    fn settle(&self, record: &IndexMap<String, Seat>, since: u64) -> Vec<Drift> {
        let mut inner = self.lock();
        let mut profiles: Vec<String> = inner
            .held
            .keys()
            .chain(record.keys())
            .cloned()
            .collect::<IndexSet<String>>()
            .into_iter()
            .collect();
        profiles.sort();
        let mut drifts = Vec::new();
        for profile in profiles {
            if inner.changed.get(&profile).is_some_and(|at| *at > since) {
                continue;
            }
            let ledger = match inner.held.get(&profile) {
                Some(held) if held.lease.is_some() => continue,
                held => held.map(|held| held.seat.clone()),
            };
            let recorded = record.get(&profile).cloned();
            if ledger == recorded {
                continue;
            }
            match &recorded {
                Some(seat) => {
                    inner.held.insert(
                        profile.clone(),
                        Held {
                            seat: seat.clone(),
                            lease: None,
                        },
                    );
                }
                None => {
                    inner.held.swap_remove(&profile);
                }
            }
            drifts.push(Drift {
                profile,
                ledger,
                record: recorded,
            });
        }
        drifts
    }
}

/// One read of the record in flight, from `Seats::mark` (R1523000). Settled or dropped, it closes,
/// and the stamps no read still open needs are let go.
#[must_use = "a reading settles the record it marked"]
pub struct Reading<'a> {
    seats: &'a Seats,
    since: u64,
}

impl Reading<'_> {
    /// The record read after the mark, against the ledger (`Seats::settle`).
    pub fn settle(self, record: &IndexMap<String, Seat>) -> Vec<Drift> {
        self.seats.settle(record, self.since)
    }
}

impl Drop for Reading<'_> {
    fn drop(&mut self) {
        let mut inner = self.seats.lock();
        if let Some(at) = inner.reading.iter().position(|since| *since == self.since) {
            inner.reading.swap_remove(at);
        }
        // A read opened later marks at the clock now or after, so it needs no stamp up to here.
        let oldest = inner.reading.iter().copied().min().unwrap_or(inner.clock);
        inner.changed.retain(|_, at| *at > oldest);
    }
}

/// The seats one `reserve` took, pending. `commit` makes them held; dropped uncommitted, it puts
/// back what each profile held before, so a door that fails before its game stands needs no
/// release of its own.
#[must_use = "a lease dropped at once gives its seats back"]
pub struct Lease<'a> {
    seats: &'a Seats,
    id: u64,
    prior: Vec<(String, Option<Seat>)>,
    done: bool,
}

impl Lease<'_> {
    /// The game stands: its seats are held until its ending releases them.
    pub fn commit(mut self) {
        let seats = self.seats;
        let mut inner = seats.lock();
        for (profile, _) in &self.prior {
            let Some(held) = inner.held.get_mut(profile) else {
                continue;
            };
            if held.lease != Some(self.id) {
                continue;
            }
            held.lease = None;
            inner.touch(profile);
        }
        drop(inner);
        self.done = true;
    }
}

impl Drop for Lease<'_> {
    fn drop(&mut self) {
        if self.done {
            return;
        }
        let (seats, id) = (self.seats, self.id);
        let mut inner = seats.lock();
        for (profile, before) in self.prior.drain(..) {
            if inner.held.get(&profile).is_none_or(|held| held.lease != Some(id)) {
                continue;
            }
            match before {
                Some(seat) => {
                    inner.held.insert(profile.clone(), Held { seat, lease: None });
                }
                None => {
                    inner.held.swap_remove(&profile);
                }
            }
            inner.touch(&profile);
        }
    }
}

/// The seats the record holds (R1523000): every set in-match flag as its match, as the
/// compare-and-set sees it; a side of a series not over as that series, on the game its flag names
/// when that is one of the series' own, and between games when it holds nothing; a person of a live
/// co-op match as that match when they hold nothing else.
pub fn held_seats(
    seated: &[SeatedProfile],
    series: &[SeriesSeats],
    duos: &[DuosSeats],
) -> IndexMap<String, Seat> {
    let mut record: IndexMap<String, Seat> = seated
        .iter()
        .map(|row| (row.profile_id.clone(), Seat::Match(row.match_id.clone())))
        .collect();
    for one in series {
        for side in &one.sides {
            let seat = match record.get(side) {
                Some(Seat::Match(game)) if one.games.contains(game) => Some(Seat::Series {
                    id: one.id.clone(),
                    game: Some(game.clone()),
                }),
                None => Some(Seat::Series {
                    id: one.id.clone(),
                    game: None,
                }),
                Some(_) => None,
            };
            if let Some(seat) = seat {
                record.insert(side.clone(), seat);
            }
        }
    }
    for one in duos {
        for person in &one.people {
            if !record.contains_key(person) {
                record.insert(person.clone(), Seat::Duos(one.id.clone()));
            }
        }
    }
    record
}

/// The record's seats, read in one transaction: the in-match flags, the series that are not over
/// and the live co-op matches.
async fn read_record(app: &App) -> Result<IndexMap<String, Seat>, StoreError> {
    let mut tx = app.db.begin(None).await?;
    let seated = tx.profiles_seated().await?;
    let series: Vec<SeriesSeats> = tx
        .series_active()
        .await?
        .into_iter()
        .map(|row| SeriesSeats {
            games: row
                .games
                .iter()
                .map(|game| game.match_id.clone())
                .chain(std::iter::once(row.next_match_id.clone()))
                .collect(),
            sides: [row.sides.0.profile_id, row.sides.1.profile_id],
            id: row.id,
        })
        .collect();
    let mut duos = Vec::new();
    for match_id in tx.duos_matches_live().await? {
        if let Some(row) = tx.duos_matches_get(&match_id).await? {
            duos.push(DuosSeats {
                id: row.id,
                people: [row.a1, row.a2],
            });
        }
    }
    tx.commit().await?;
    Ok(held_seats(&seated, &series, &duos))
}

/// Boot (R1523000), before the port opens and before any loop starts: the epoch moves on, which
/// fences every claim an older process still has in flight, and only then is the ledger filled from
/// the record, which so holds every seat that process committed.
pub async fn boot(app: &App) -> Result<(), StoreError> {
    let epoch = one_tx!(app.db, |t| t.seats_bump_epoch().await?)?;
    app.seats.set_epoch(epoch);
    hydrate(app).await?;
    tracing::info!(event = "seats.booted", epoch = epoch);
    Ok(())
}

/// Fills the ledger from the record, as boot does after its bump.
pub async fn hydrate(app: &App) -> Result<(), StoreError> {
    let record = read_record(app).await?;
    app.seats.fill(record);
    Ok(())
}

/// The reaper's tick (R1523000): the ledger against the record. A seat whose game ended without its
/// release is freed and a seat the record holds that the ledger lacks is adopted, each logged as
/// `seats.drift`; what changed while the record was being read waits for the next tick.
pub async fn reconcile(app: &App) {
    let reading = app.seats.mark();
    let record = match read_record(app).await {
        Ok(record) => record,
        Err(error) => {
            tracing::warn!(event = "seats.reconcile_failed", message = %error);
            return;
        }
    };
    for drift in reading.settle(&record) {
        tracing::warn!(
            event = "seats.drift",
            source = "reconcile",
            profileId = %drift.profile,
            ledger = ?drift.ledger,
            record = ?drift.record,
        );
    }
}

/// A series that is over lets both its sides go.
pub fn free_series(app: &App, series: &SeriesRow) {
    if series.status != SeriesStatus::Over {
        return;
    }
    let seat = Seat::Series {
        id: series.id.clone(),
        game: None,
    };
    for side in [&series.sides.0, &series.sides.1] {
        app.seats.release(&side.profile_id, &seat);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn game(id: &str) -> Seat {
        Seat::Match(id.to_string())
    }

    fn series(id: &str, game: Option<&str>) -> Seat {
        Seat::Series {
            id: id.to_string(),
            game: game.map(str::to_string),
        }
    }

    fn taken(profile: &str, seat: Seat) -> Result<(), Taken> {
        Err(Taken {
            profile: profile.to_string(),
            seat,
        })
    }

    /// `reserve`'s answer with the lease committed at once.
    fn held(seats: &Seats, profiles: &[&str], seat: Seat) {
        seats
            .reserve(profiles, seat)
            .expect("the seats are free")
            .commit();
    }

    #[test]
    fn r1523000_a_reservation_is_all_or_nothing() {
        let seats = Seats::default();
        held(&seats, &["b"], game("m1"));
        assert_eq!(
            seats.reserve(&["a", "b"], game("m2")).map(drop),
            taken("b", game("m1"))
        );
        assert_eq!(seats.of("a"), None, "a refused reservation reserves nobody");
        assert_eq!(seats.of("b"), Some(game("m1")));

        let lease = seats.reserve(&["a", "c"], game("m3")).expect("both are free");
        assert_eq!(
            (seats.of("a"), seats.of("c")),
            (Some(game("m3")), Some(game("m3")))
        );
        lease.commit();
        assert_eq!(
            (seats.of("a"), seats.of("c")),
            (Some(game("m3")), Some(game("m3")))
        );
    }

    #[test]
    fn r1523000_a_pending_seat_blocks_a_rival() {
        let seats = Seats::default();
        let lease = seats
            .reserve(&["a", "host"], Seat::Duos("d1".to_string()))
            .expect("free");
        assert_eq!(
            seats.of("host"),
            Some(Seat::Duos("d1".to_string())),
            "a pending seat is read"
        );
        assert_eq!(
            seats.reserve(&["b", "host"], game("m1")).map(drop),
            taken("host", Seat::Duos("d1".to_string()))
        );
        assert_eq!(
            seats.reserve(&["host"], Seat::Duos("d1".to_string())).map(drop),
            taken("host", Seat::Duos("d1".to_string())),
            "not even the same seat while it is pending"
        );
        assert_eq!(seats.of("b"), None);
        drop(lease);
    }

    #[test]
    fn r1523000_dropping_a_lease_releases_it() {
        let seats = Seats::default();
        drop(seats.reserve(&["a", "b"], game("m1")).expect("free"));
        assert_eq!((seats.of("a"), seats.of("b")), (None, None));
        held(&seats, &["a", "b"], game("m2"));

        // A series between games takes its next game and gets back to between games when that
        // game's start fails.
        held(&seats, &["c", "d"], series("s1", None));
        let next = seats
            .reserve(&["c", "d"], series("s1", Some("g1")))
            .expect("its next game");
        assert_eq!(seats.of("c"), Some(series("s1", Some("g1"))));
        drop(next);
        assert_eq!(
            (seats.of("c"), seats.of("d")),
            (Some(series("s1", None)), Some(series("s1", None)))
        );
    }

    #[test]
    fn r1523000_release_is_conditional() {
        let seats = Seats::default();
        held(&seats, &["a"], game("m1"));
        seats.release("a", &game("m2"));
        assert_eq!(
            seats.of("a"),
            Some(game("m1")),
            "another match's release leaves it"
        );
        seats.release("a", &Seat::Duos("m1".to_string()));
        assert_eq!(
            seats.of("a"),
            Some(game("m1")),
            "another kind of seat's release leaves it"
        );
        seats.release("a", &game("m1"));
        assert_eq!(seats.of("a"), None);

        let lease = seats.reserve(&["b"], game("m3")).expect("free");
        seats.release("b", &game("m3"));
        assert_eq!(
            seats.of("b"),
            Some(game("m3")),
            "a pending seat is the door's to give back"
        );
        lease.commit();

        held(&seats, &["c"], series("s1", None));
        let next = seats
            .reserve(&["c"], series("s1", Some("g1")))
            .expect("its next game");
        next.commit();
        seats.release("c", &series("s2", None));
        assert_eq!(seats.of("c"), Some(series("s1", Some("g1"))));
        seats.release("c", &series("s1", None));
        assert_eq!(seats.of("c"), None, "a series lets go whatever game it is on");
    }

    #[test]
    fn r1523000_a_series_admits_its_own_next_game() {
        let seats = Seats::default();
        held(&seats, &["a", "b"], series("s1", None));
        assert_eq!(
            seats.reserve(&["a"], series("s2", Some("g9"))).map(drop),
            taken("a", series("s1", None)),
            "not another series' game"
        );
        assert_eq!(
            seats.reserve(&["a"], game("g1")).map(drop),
            taken("a", series("s1", None)),
            "not a match"
        );
        held(&seats, &["a", "b"], series("s1", Some("g1")));
        assert_eq!(
            seats.reserve(&["a"], series("s1", Some("g2"))).map(drop),
            taken("a", series("s1", Some("g1"))),
            "not a second game while one is played"
        );

        seats.end_game("a", "g9");
        assert_eq!(
            seats.of("a"),
            Some(series("s1", Some("g1"))),
            "another match's end leaves it"
        );
        seats.end_game("a", "g1");
        seats.end_game("b", "g1");
        assert_eq!(
            (seats.of("a"), seats.of("b")),
            (Some(series("s1", None)), Some(series("s1", None)))
        );
        held(&seats, &["a", "b"], series("s1", Some("g2")));
        assert_eq!(seats.of("b"), Some(series("s1", Some("g2"))));
    }

    #[test]
    fn r1523000_a_match_admits_its_own_retry() {
        let seats = Seats::default();
        let first = seats.reserve(&["bot"], game("m1")).expect("free");
        assert_eq!(
            seats.reserve(&["a", "bot"], game("m1")).map(drop),
            taken("bot", game("m1")),
            "not while it is pending"
        );
        first.commit();
        let retry = seats
            .reserve(&["a", "bot"], game("m1"))
            .expect("its own match again");
        drop(retry);
        assert_eq!(
            seats.of("bot"),
            Some(game("m1")),
            "a retry given back leaves the seat held"
        );
        assert_eq!(seats.of("a"), None);
        assert_eq!(
            seats.reserve(&["bot"], game("m2")).map(drop),
            taken("bot", game("m1"))
        );
        seats.end_game("bot", "m1");
        assert_eq!(seats.of("bot"), None);
    }

    #[test]
    fn r1523000_settle_frees_and_adopts_but_leaves_what_moved_since_its_read() {
        let seats = Seats::default();
        held(&seats, &["ended"], game("m1"));
        held(&seats, &["moved"], game("m2"));
        let pending = seats.reserve(&["pending"], game("m3")).expect("free");
        let reading = seats.mark();
        // After the mark, as a door's commit or an ending lands while the record is read.
        seats.end_game("moved", "m2");

        let record: IndexMap<String, Seat> = [
            ("moved".to_string(), game("m2")),
            ("adopted".to_string(), Seat::Duos("d1".to_string())),
        ]
        .into_iter()
        .collect();
        let drifts = reading.settle(&record);
        assert_eq!(
            drifts,
            vec![
                Drift {
                    profile: "adopted".to_string(),
                    ledger: None,
                    record: Some(Seat::Duos("d1".to_string())),
                },
                Drift {
                    profile: "ended".to_string(),
                    ledger: Some(game("m1")),
                    record: None,
                },
            ]
        );
        assert_eq!(seats.of("ended"), None);
        assert_eq!(seats.of("adopted"), Some(Seat::Duos("d1".to_string())));
        assert_eq!(
            seats.of("moved"),
            None,
            "what moved after the mark waits for the next tick"
        );
        assert_eq!(
            seats.of("pending"),
            Some(game("m3")),
            "a pending seat is left alone"
        );
        pending.commit();

        // The next tick, on a record read after everything above.
        let reading = seats.mark();
        let record: IndexMap<String, Seat> = [
            ("adopted".to_string(), Seat::Duos("d1".to_string())),
            ("pending".to_string(), game("m3")),
        ]
        .into_iter()
        .collect();
        assert_eq!(reading.settle(&record), Vec::new());
    }

    #[test]
    fn r1523000_an_overlapping_reconcile_keeps_what_moved_after_the_older_read() {
        let seats = Seats::default();
        // The reaper's tick marks and starts reading: the person holds nothing yet.
        let older = seats.mark();
        // A door seats the person meanwhile.
        held(&seats, &["person"], Seat::Duos("d1".to_string()));
        // The matchmaker's release reconciles too, reads the record with the seat and settles first.
        let newer = seats.mark();
        let seated: IndexMap<String, Seat> = [("person".to_string(), Seat::Duos("d1".to_string()))]
            .into_iter()
            .collect();
        assert_eq!(newer.settle(&seated), Vec::new());

        // The older read, from before the door, settles last and must not free the live seat.
        assert_eq!(older.settle(&IndexMap::new()), Vec::new());
        assert_eq!(seats.of("person"), Some(Seat::Duos("d1".to_string())));

        // With no read open, the stamps go, and a fresh read settles the record again.
        assert!(seats.lock().changed.is_empty());
        let fresh = seats.mark();
        assert_eq!(
            fresh.settle(&IndexMap::new()),
            vec![Drift {
                profile: "person".to_string(),
                ledger: Some(Seat::Duos("d1".to_string())),
                record: None,
            }]
        );
    }

    #[test]
    fn r1523000_a_reconcile_open_across_a_fill_leaves_what_the_fill_wrote() {
        let seats = Seats::default();
        held(&seats, &["gone"], game("m1"));
        let older = seats.mark();
        seats.fill([("filled".to_string(), game("m2"))].into_iter().collect());
        let stale: IndexMap<String, Seat> = [("gone".to_string(), game("m1"))].into_iter().collect();
        assert_eq!(older.settle(&stale), Vec::new());
        assert_eq!(seats.of("gone"), None);
        assert_eq!(seats.of("filled"), Some(game("m2")));
    }

    #[test]
    fn r1523000_held_seats_reads_the_record() {
        let seated = |profile: &str, match_id: &str| SeatedProfile {
            profile_id: profile.to_string(),
            match_id: match_id.to_string(),
        };
        let record = held_seats(
            &[
                seated("solo", "m1"),
                seated("playing", "g2"),
                seated("elsewhere", "m9"),
                seated("both", "m5"),
            ],
            &[
                SeriesSeats {
                    id: "s1".to_string(),
                    sides: ["playing".to_string(), "picking".to_string()],
                    games: vec!["g1".to_string(), "g2".to_string()],
                },
                SeriesSeats {
                    id: "s2".to_string(),
                    sides: ["elsewhere".to_string(), "between".to_string()],
                    games: vec!["g7".to_string()],
                },
            ],
            &[DuosSeats {
                id: "d1".to_string(),
                people: ["both".to_string(), "duo".to_string()],
            }],
        );
        let expected: IndexMap<String, Seat> = [
            ("solo", game("m1")),
            ("playing", series("s1", Some("g2"))),
            ("elsewhere", game("m9")),
            ("both", game("m5")),
            ("picking", series("s1", None)),
            ("between", series("s2", None)),
            ("duo", Seat::Duos("d1".to_string())),
        ]
        .into_iter()
        .map(|(profile, seat)| (profile.to_string(), seat))
        .collect();
        assert_eq!(record, expected);
    }
}
