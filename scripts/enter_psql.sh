#!bin/bash

# This script automatically logs into PostgreSQL as an admin user.
# WARNINNG: This script should be used for development purposes ONLY!

# Import a function for importing environmental variables.
source ./scripts/helpers/load_env.sh

# Load environmental variables.
load_env '.env'

# Log into PostgreSQL as an admin user.
PGPASSWORD=$POSTGRES_PASSWORD psql -U $POSTGRES_USER -h localhost -d $POSTGRES_DB

