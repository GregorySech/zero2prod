-- Start migration transaction.
BEGIN;
    -- Setting confirmed status as if all came from subscriptions::insert_subscriber.
    UPDATE subscriptions
    SET status = 'confirmed'
    WHERE status IS NULL;

    -- Make status column mandatory.
    ALTER TABLE subscriptions ALTER COLUMN status SET NOT NULL;
COMMIT;