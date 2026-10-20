"""The integer network (#1011): it stays close to the float one it was made from, its weights fit
their integers, and the parity fixture is what its reference computes."""

import json
import math
from pathlib import Path

from jackioh_ml import export as E
from jackioh_ml import quantize as Q

FIXTURE = Path(__file__).parent / "fixtures" / "positions.jsonl"
PARITY = Path(__file__).resolve().parents[3] / "crates" / "ai" / "tests" / "fixtures" / "learned-parity.json"


def _rows(count: int):
    lines = FIXTURE.read_text(encoding="utf-8").splitlines()[1 : count + 1]
    return [json.loads(line) for line in lines]


def test_the_integer_network_stays_close_to_the_float_one():
    float_weights = E.random_network(FIXTURE, 1011)
    int_weights = Q.quantize(float_weights)
    net = Q.unpacked(int_weights)
    worst = 0.0
    for row in _rows(100):
        a = E.correction(float_weights, row["dense"], row["sparse"])
        b = Q.correction(net, row["dense"], row["sparse"])
        worst = max(worst, abs(a - b))
    # A logit of 0.05 is a few hundredths of the hand eval's points per its unit.
    assert worst < 0.05, worst


def test_every_weight_fits_its_integer():
    int_weights = Q.quantize(E.random_network(FIXTURE, 7))
    net = Q.unpacked(int_weights)
    assert all(-32768 <= w <= 32767 for w in net["denseIn"] + net["sparseIn"])
    for _, shift, w, _ in net["head"]:
        assert shift >= 0 and all(-128 <= x <= 127 for x in w)
    shift, w, _ = net["out"]
    assert shift >= 0 and all(-128 <= x <= 127 for x in w)
    assert int_weights["format"] == Q.FORMAT and int_weights["activation"] == Q.ACTIVATION


def test_the_committed_parity_fixture_is_what_the_reference_computes():
    fixture = json.loads(PARITY.read_text(encoding="utf-8"))
    net = Q.unpacked(fixture["network"])
    for case in fixture["cases"]:
        value = Q.correction(net, case["dense"], case["sparse"])
        assert repr(value) == case["expected"]
        assert math.isfinite(value)
    # And it is the fixture `--random 1011` writes: the seeded network over the training fixture.
    again = Q.parity(Q.quantize(E.random_network(FIXTURE, 1011)), FIXTURE, len(fixture["cases"]))
    assert again == fixture


def test_features_of_another_layout_get_no_correction():
    net = Q.unpacked(Q.quantize(E.random_network(FIXTURE, 3)))
    row = _rows(1)[0]
    assert Q.correction(net, row["dense"][:-1], row["sparse"]) == 0.0


def test_a_rare_features_weight_past_every_shift_is_clipped_not_refused():
    # #466: run 5's export holds a keyword on about four positions, a spread of 0.002; folded as
    # `W/scale`, a weight of 1 on it is 500, and 500·127 is past the i16 first layer at shift 0.
    float_weights = E.random_network(FIXTURE, 7)
    float_weights["scale"] = list(float_weights["scale"])
    float_weights["scale"][0] = 0.002
    float_weights["denseIn"] = [list(row) for row in float_weights["denseIn"]]
    float_weights["denseIn"][0][0] = 1.0
    assert Q.clipped(float_weights) >= 1
    net = Q.unpacked(Q.quantize(float_weights))
    assert net["shift"] == 0
    assert all(-32768 <= w <= 32767 for w in net["denseIn"] + net["sparseIn"])
    assert 32767 in net["denseIn"]
    assert Q.clipped(E.random_network(FIXTURE, 7)) == 0


def test_a_won_or_lost_positions_logit_in_the_tens_of_thousands_has_the_clipped_loss():
    # The hand eval's lethal constants put such a logit far past what math.exp takes (#466).
    assert Q.log_loss(-5000.0, 0.0) == Q.log_loss(-20.0, 0.0)
    assert Q.log_loss(-5000.0, 1.0) == -math.log(1e-7)
    assert Q.log_loss(5000.0, 1.0) == Q.log_loss(20.0, 1.0)
    assert abs(Q.log_loss(0.0, 1.0) - math.log(2)) < 1e-12
