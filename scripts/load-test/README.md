# ProxyPal Load Testing

Load testing scripts using [k6](https://k6.io/) for proxypal-server.

## Prerequisites

1. Install k6:
   ```bash
   # Windows (winget)
   winget install k6

   # Windows (choco)
   choco install k6

   # macOS
   brew install k6

   # Linux (Debian/Ubuntu)
   sudo apt-key adv --keyserver hkp://keyserver.ubuntu.com:80 --recv-keys C5AD17C747E3415A3642D57D77C6C491D6AC1D69
   echo "deb https://dl.k6.io/deb stable main" | sudo tee /etc/apt/sources.list.d/k6.list
   sudo apt-get update && sudo apt-get install k6
   ```

2. Ensure proxypal-server is running on port 3000

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `BASE_URL` | `http://localhost:3000` | Server base URL |
| `ADMIN_PASSWORD` | `test123` | Admin password for login |
| `API_KEYS` | `key1,key2,key3,key4,key5` | Comma-separated list of user API keys |

## Running Tests

### Admin API Test

Tests login, user creation, and stats endpoints with 5 VUs for 1 minute.

```bash
k6 run scripts/load-test/admin-api.js
```

With custom settings:
```bash
k6 run -e BASE_URL=http://localhost:3000 -e ADMIN_PASSWORD=mypassword scripts/load-test/admin-api.js
```

### Proxy API Test

Tests `/v1/chat/completions` endpoint with 5 VUs for 2 minutes.

```bash
k6 run scripts/load-test/proxy-api.js
```

With API keys from your test users:
```bash
k6 run -e API_KEYS="pp_abc123,pp_def456,pp_ghi789" scripts/load-test/proxy-api.js
```

### Full Load Test

Combined test with ramp-up/down (30s → 2m → 30s).

```bash
k6 run scripts/load-test/full-load.js
```

## Interpreting Results

### Key Metrics

| Metric | Description | Target |
|--------|-------------|--------|
| `http_req_duration` | Request latency | p95 < 500ms (admin), < 2000ms (proxy) |
| `errors` | Error rate | < 1% (admin), < 5% (proxy) |
| `http_reqs` | Requests per second | Baseline metric |

### Threshold Results

- **✓ PASS**: Metric meets the defined threshold
- **✗ FAIL**: Metric exceeds the threshold

### Example Output

```
✓ http_req_duration.............: avg=123.45ms p(95)=456.78ms
✓ errors........................: 0.50%
  http_reqs.....................: 500    8.33/s
```

## Expected Baseline Metrics

For a healthy proxypal-server with 5 concurrent users:

| Endpoint | Expected p95 | Expected RPS |
|----------|--------------|--------------|
| POST /api/auth/login | < 100ms | ~10 |
| GET /api/users | < 50ms | ~20 |
| GET /api/stats | < 50ms | ~20 |
| POST /v1/chat/completions | < 2000ms* | ~2-5 |

*Proxy latency depends on upstream provider response time.

## Tips

1. **Warm up the server** before running load tests
2. **Monitor server resources** (CPU, memory) during tests
3. **Run multiple iterations** to get consistent baselines
4. **Export results** for comparison:
   ```bash
   k6 run --out json=results.json scripts/load-test/full-load.js
   ```

## Troubleshooting

### "connection refused" errors
- Ensure proxypal-server is running
- Check BASE_URL is correct

### High error rates on proxy API
- Verify API_KEYS are valid user keys from the database
- Check upstream provider is accessible

### Rate limiting (429 errors)
- Expected behavior for proxy tests
- Not counted as errors in the metrics
