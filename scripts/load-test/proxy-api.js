import http from "k6/http";
import { check, sleep } from "k6";
import { Rate, Trend } from "k6/metrics";

const errorRate = new Rate("errors");
const proxyDuration = new Trend("proxy_duration");

const BASE_URL = __ENV.BASE_URL || "http://localhost:3000";
const API_KEYS = (__ENV.API_KEYS || "key1,key2,key3,key4,key5").split(",");

export const options = {
  vus: 5,
  duration: "2m",
  thresholds: {
    http_req_duration: ["p(95)<2000"],
    errors: ["rate<0.05"],
  },
};

const chatPayload = JSON.stringify({
  model: "gpt-4o-mini",
  messages: [
    { role: "system", content: "You are a helpful assistant." },
    { role: "user", content: "Hello, how are you?" },
  ],
  max_tokens: 100,
  temperature: 0.7,
});

export default function () {
  const apiKey = API_KEYS[__VU % API_KEYS.length];

  const headers = {
    "Content-Type": "application/json",
    Authorization: `Bearer ${apiKey}`,
  };

  const start = Date.now();
  const res = http.post(`${BASE_URL}/v1/chat/completions`, chatPayload, {
    headers,
    timeout: "30s",
  });
  proxyDuration.add(Date.now() - start);

  const success = check(res, {
    "proxy status 200": (r) => r.status === 200,
    "proxy status not 500": (r) => r.status !== 500,
    "response has choices": (r) => {
      try {
        const body = JSON.parse(r.body);
        return body.choices !== undefined || body.error !== undefined;
      } catch {
        return false;
      }
    },
  });

  if (res.status === 429) {
    console.log(`VU ${__VU}: Rate limited`);
  }

  errorRate.add(!success && res.status !== 429);

  sleep(Math.random() * 2 + 1);
}

export function handleSummary(data) {
  return {
    stdout: textSummary(data, { indent: "  ", enableColors: true }),
  };
}

function textSummary(data, opts) {
  const metrics = data.metrics;
  let summary = "\n=== Proxy API Load Test Summary ===\n\n";

  if (metrics.http_req_duration) {
    summary += `Request Duration:\n`;
    summary += `  avg: ${metrics.http_req_duration.values.avg.toFixed(2)}ms\n`;
    summary += `  p95: ${metrics.http_req_duration.values["p(95)"].toFixed(2)}ms\n`;
    summary += `  max: ${metrics.http_req_duration.values.max.toFixed(2)}ms\n\n`;
  }

  if (metrics.errors) {
    summary += `Error Rate: ${(metrics.errors.values.rate * 100).toFixed(2)}%\n`;
  }

  if (metrics.http_reqs) {
    summary += `Total Requests: ${metrics.http_reqs.values.count}\n`;
    summary += `Requests/sec: ${metrics.http_reqs.values.rate.toFixed(2)}\n`;
  }

  return summary;
}
