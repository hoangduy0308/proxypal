import http from "k6/http";
import { check, sleep, group } from "k6";
import { Rate, Trend, Counter } from "k6/metrics";

const errorRate = new Rate("errors");
const adminOps = new Counter("admin_operations");
const proxyOps = new Counter("proxy_operations");
const adminDuration = new Trend("admin_duration");
const proxyDuration = new Trend("proxy_duration");

const BASE_URL = __ENV.BASE_URL || "http://localhost:3000";
const ADMIN_PASSWORD = __ENV.ADMIN_PASSWORD || "test123";
const API_KEYS = (__ENV.API_KEYS || "key1,key2,key3,key4,key5").split(",");

export const options = {
  stages: [
    { duration: "30s", target: 5 },
    { duration: "2m", target: 5 },
    { duration: "30s", target: 0 },
  ],
  thresholds: {
    http_req_duration: ["p(95)<1000"],
    errors: ["rate<0.05"],
    admin_duration: ["p(95)<500"],
    proxy_duration: ["p(95)<2000"],
  },
};

let adminToken = null;

export function setup() {
  const loginRes = http.post(
    `${BASE_URL}/api/auth/login`,
    JSON.stringify({ password: ADMIN_PASSWORD }),
    { headers: { "Content-Type": "application/json" } }
  );

  if (loginRes.status !== 200) {
    console.error("Failed to login during setup");
    return { token: null };
  }

  const body = JSON.parse(loginRes.body);
  return { token: body.token };
}

export default function (data) {
  const scenario = Math.random();

  if (scenario < 0.3) {
    adminScenario(data);
  } else {
    proxyScenario();
  }
}

function adminScenario(data) {
  group("Admin API", function () {
    const headers = {
      "Content-Type": "application/json",
      Authorization: `Bearer ${data.token}`,
    };

    const start = Date.now();

    const usersRes = http.get(`${BASE_URL}/api/users`, { headers });
    const usersSuccess = check(usersRes, {
      "list users ok": (r) => r.status === 200,
    });

    const statsRes = http.get(`${BASE_URL}/api/stats`, { headers });
    const statsSuccess = check(statsRes, {
      "get stats ok": (r) => r.status === 200,
    });

    adminDuration.add(Date.now() - start);
    adminOps.add(2);
    errorRate.add(!usersSuccess || !statsSuccess);

    sleep(1);
  });
}

function proxyScenario() {
  group("Proxy API", function () {
    const apiKey = API_KEYS[__VU % API_KEYS.length];

    const headers = {
      "Content-Type": "application/json",
      Authorization: `Bearer ${apiKey}`,
    };

    const payload = JSON.stringify({
      model: "gpt-4o-mini",
      messages: [
        { role: "user", content: `Test message from VU ${__VU} at ${Date.now()}` },
      ],
      max_tokens: 50,
    });

    const start = Date.now();
    const res = http.post(`${BASE_URL}/v1/chat/completions`, payload, {
      headers,
      timeout: "30s",
    });
    proxyDuration.add(Date.now() - start);
    proxyOps.add(1);

    const success = check(res, {
      "proxy response ok": (r) => r.status === 200 || r.status === 429,
    });

    errorRate.add(!success);

    sleep(Math.random() * 2 + 0.5);
  });
}

export function teardown(data) {
  console.log("\n=== Full Load Test Complete ===");
}

export function handleSummary(data) {
  const metrics = data.metrics;

  let summary = "\n" + "=".repeat(50) + "\n";
  summary += "       FULL LOAD TEST SUMMARY\n";
  summary += "=".repeat(50) + "\n\n";

  summary += "OVERALL METRICS:\n";
  if (metrics.http_reqs) {
    summary += `  Total Requests: ${metrics.http_reqs.values.count}\n`;
    summary += `  Requests/sec:   ${metrics.http_reqs.values.rate.toFixed(2)}\n`;
  }
  if (metrics.errors) {
    summary += `  Error Rate:     ${(metrics.errors.values.rate * 100).toFixed(2)}%\n`;
  }

  summary += "\nADMIN API:\n";
  if (metrics.admin_operations) {
    summary += `  Operations: ${metrics.admin_operations.values.count}\n`;
  }
  if (metrics.admin_duration) {
    summary += `  p95 Duration: ${metrics.admin_duration.values["p(95)"].toFixed(2)}ms\n`;
  }

  summary += "\nPROXY API:\n";
  if (metrics.proxy_operations) {
    summary += `  Operations: ${metrics.proxy_operations.values.count}\n`;
  }
  if (metrics.proxy_duration) {
    summary += `  p95 Duration: ${metrics.proxy_duration.values["p(95)"].toFixed(2)}ms\n`;
  }

  summary += "\nTHRESHOLDS:\n";
  for (const [name, threshold] of Object.entries(data.thresholds || {})) {
    const status = threshold.ok ? "✓ PASS" : "✗ FAIL";
    summary += `  ${name}: ${status}\n`;
  }

  summary += "\n" + "=".repeat(50) + "\n";

  return {
    stdout: summary,
    "load-test-summary.json": JSON.stringify(data, null, 2),
  };
}
