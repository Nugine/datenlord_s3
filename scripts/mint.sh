docker run \
    -e "SERVER_ENDPOINT=localhost:8014" -e "DOMAIN=localhost"  \
    -e "ACCESS_KEY=AKIAIOSFODNN7EXAMPLE" -e "SECRET_KEY=wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY" \
    -e "ENABLE_HTTPS=0" -e "ENABLE_VIRTUAL_STYLE=1" \
    minio/mint
