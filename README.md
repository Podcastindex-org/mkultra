# mkultra
An live chat server for podcasters with throwaway, carryout or BYO identities.

This software is alpha.  Do not expect the instructions to make sense at this stage.

# Description
An mkultra server has three parts:

- Websocket server
- Web server
- Database

After compiling with `cargo build`, you spin up the websocket server and the web server:

```bash
./target/debug/mkultra-socket 2083
```

```bash
./target/debug/mkultra-web 8080 `openssl rand -hex 21`
```

These should both be behind an nginx reverse proxy with appropriate TLS certs:

```nginx
server {
        listen [::]:8443 ssl ipv6only=on; # managed by Certbot
        listen 8443 ssl; # managed by Certbot

        root /var/www/html;

        index index.html index.htm index.nginx-debian.html;

        server_name [servername]; # managed by Certbot

        location / {
            proxy_set_header Upgrade $http_upgrade;
            proxy_set_header Connection "Upgrade";
            proxy_http_version 1.1;
            proxy_set_header Host $host;
            proxy_pass http://127.0.0.1:2083;
        }

        ssl_certificate /etc/letsencrypt/live/[servername]/fullchain.pem; # managed by Certbot
        ssl_certificate_key /etc/letsencrypt/live/[servername]/privkey.pem; # managed by Certbot
        include /etc/letsencrypt/options-ssl-nginx.conf; # managed by Certbot
        ssl_dhparam /etc/letsencrypt/ssl-dhparams.pem; # managed by Certbot
}
```
