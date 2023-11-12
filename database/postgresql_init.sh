# This script initializes postgresql and makes some init injections into the database.

#!/bin/bash

# Replace placeholders in the SQL template with environment variable values.
envsubst < /docker-entrypoint-initdb.d/99_injections.sql.template > /docker-entrypoint-initdb.d/99_injections.sql

# Execute the generated SQL file.
psql -U "$POSTGRES_USER" -d "$POSTGRES_DB" -f /docker-entrypoint-initdb.d/99_injections.sql
