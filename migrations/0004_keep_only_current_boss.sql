DELETE FROM simulation_jobs;

DELETE FROM raid_bosses
WHERE id NOT IN (
    SELECT id FROM raid_bosses ORDER BY active DESC, updated_at DESC LIMIT 1
);

DELETE FROM raids
WHERE NOT EXISTS (SELECT 1 FROM raid_bosses WHERE raid_bosses.raid_id = raids.id);

UPDATE raid_bosses SET active = TRUE;
