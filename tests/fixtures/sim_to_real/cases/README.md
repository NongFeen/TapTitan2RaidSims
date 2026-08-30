# Sim-to-real golden test cases

Every `.toml` file in this directory is loaded and run -- split them however
is useful (one per boss, per deck, per mechanic being exercised, whatever
makes it easy to find things). Each file can hold one or more `[[case]]`
entries.

Each case runs one deterministic attack -- exactly one tap, guaranteed to
proc every card with a nonzero proc chance, then the rest of a 600-tick
round plays out with no further taps (so affliction/DoT cards finish
ticking) -- and compares the result, component by component, to damage
numbers measured by hand in real TapTitan2. See
`SimService::run_deterministic_single_tap_simulation` for the mechanism.

## Editor autocomplete / validation

Every case file should start with `#:schema ./cases.schema.json` (see the
example below) -- that's a magic comment the **Even Better TOML** VS Code
extension (`tamasfe.even-better-toml`) reads to attach a JSON Schema to the
file. With it installed you get real autocomplete and a red squiggly on
typos for `deck`/`attackable_part` card ids and part names, instead of a
silent misspelling that just fails at test time.
`cases.schema.json`'s `enum` lists are generated from `CardName`'s
`#[serde(rename = ...)]` ids and `BossPartName`'s variants in
`models/cards.rs`/`models/boss/parts.rs` -- if a new card is ever added
there, add its id to the schema's `card_name` enum too. Plain TOML parsers
(including the one this test actually uses) ignore `#:schema` as an
ordinary comment, so it has zero effect on the test itself.

## Fields

- **name** -- Short, unique label for this case across *all* files (shown on
  failure).
- **payload** -- Path, relative to `tests/fixtures/sim_to_real/` (the parent
  of this `cases/` directory, not this directory itself), to a fixture
  shaped `{ "player_raw_data": <raw TT2 player export>, "boss_data": <Boss
  JSON snapshot>, "title": <f32> }` -- `player_raw_data` is the same raw
  export shape the app normally runs through `clean_data` on import,
  `boss_data` is a direct `Boss` struct for boss state (part HP/armor/curse
  at the moment of the attack), and `title` is `PlayerRaidData::title` (an
  additive bonus to all raid damage, see `damage_cache.rs`'s
  `raid_all_mult`) -- it's supplied separately because it isn't part of the
  raw export `clean_data` reads, and defaults to `0.0` if omitted. One
  payload file can be reused across many deck variants/cases/files.
- **deck** -- Exactly 3 card ids, all present (and thus enabled) in the
  payload's `raidCards`. NOTE: a card's id here is its *serialized* id,
  which is not always its display name -- e.g. "Radiant Kaleidoscope" is
  `"TriangleSupport"`. Check `CardName`'s `#[serde(rename = ...)]` in
  `models/cards.rs` if unsure.
- **attackable_part** -- Which part was targeted for this attack. Keep it to
  exactly the one real target -- with more than one attackable part,
  different attack patterns could pick different targets on the first tap.
- **expected_tap_damage** -- Real damage measured in-game from the player's
  own tap alone, excluding every card's contribution.
- **expected_card_damage** -- Real per-card damage measured in-game,
  *positional*: `expected_card_damage[i]` is `deck[i]`'s damage, so it must
  be the same length as `deck` (3 entries). Not keyed by card name -- that
  was the previous format, dropped because it let `deck` and
  `expected_card_damage` silently drift out of sync (e.g. copy-pasting a
  case as a template for a new deck and forgetting to update the keys). A
  card with no measurable contribution (e.g. a Support card, or one that
  just can't proc here) should still have an entry, `0`.
- **mirror_force_boost** -- Optional, default `0.0`. Fractional clan boost
  (`0.35` = Mirror Force deals 35% more).

No `tolerance_percent` field: TT2 only ever shows damage to 2 decimals of a
K/M/B/T-shortened number, so every expected value here is already lossy and
comparing raw digits below that precision would just be noise. The test
itself truncates both the simulated and expected numbers to that same
2-decimal display precision before comparing (TT2 truncates, not rounds),
instead of a percentage fuzz band -- see `truncate_to_display_precision` in
`sim_to_real_tests.rs`.

## Example

```toml
#:schema ./cases.schema.json

[[case]]
name = "example_single_card"
payload = "example_payload.json"
deck = ["CardA", "CardB", "CardC"]
attackable_part = ["Head"]
expected_tap_damage = 4610
expected_card_damage = [123456, 654321, 0]  # CardA, CardB, CardC, in deck order
```
