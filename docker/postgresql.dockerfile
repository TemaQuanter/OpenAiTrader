FROM postgres:15

# Install gettext for envsubst
RUN apt-get update && \
    apt-get install -y gettext-base && \
    rm -rf /var/lib/apt/lists/*

# Copy initialization scripts
COPY ./database /docker-entrypoint-initdb.d/

# Change permissions to allow script execution and file creation
# Set the script to be executable and change ownership to postgres user
RUN chmod -R 755 /docker-entrypoint-initdb.d/ && \
    chown -R postgres:postgres /docker-entrypoint-initdb.d/

# Set environment variables
ENV POSTGRES_USER=$POSTGRES_USER
ENV POSTGRES_PASSWORD=$POSTGRES_PASSWORD
ENV POSTGRES_DB=$POSTGRES_DB
