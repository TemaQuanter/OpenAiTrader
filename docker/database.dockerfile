FROM postgres:15

# Set environment variables
ENV POSTGRES_USER=$POSTGRES_USER
ENV POSTGRES_PASSWORD=$POSTGRES_PASSWORD
ENV POSTGRES_DB=$POSTGRES_DB

# Add initialization scripts
COPY ./database /docker-entrypoint-initdb.d/
