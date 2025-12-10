import http from "k6/http";
import { check, sleep } from "k6";
import { Rate, Trend } from "k6/metrics";

const errorRate = new Rate("errors");
const loginDuration = new Trend("login_duration");
const createUserDuration = new Trend("create_user_duration");

const BASE_URL = __ENV.BASE_URL || "http://localhost:3000";
const ADMIN_PASSWORD = __ENV.ADMIN_PASSWORD || "test123";

export const options = {
  vus: 5,
  duration: "1m",
  thresholds: {
    http_req_duration: ["p(95)<500"],
    errors: ["rate<0.01"],
  },
};

let authToken = null;

export function setup() {
  const loginRes = http.post(
    `${BASE_URL}/api/auth/login`,
    JSON.stringify({ password: ADMIN_PASSWORD }),
    { headers: { "Content-Type": "application/json" } }
  );

  check(loginRes, {
    "login successful": (r) => r.status === 200,
  });

  const body = JSON.parse(loginRes.body);
  return { token: body.token };
}

export default function (data) {
  const headers = {
    "Content-Type": "application/json",
    Authorization: `Bearer ${data.token}`,
  };

  // Test login endpoint
  const loginStart = Date.now();
  const loginRes = http.post(
    `${BASE_URL}/api/auth/login`,
    JSON.stringify({ password: ADMIN_PASSWORD }),
    { headers: { "Content-Type": "application/json" } }
  );
  loginDuration.add(Date.now() - loginStart);

  const loginSuccess = check(loginRes, {
    "login status 200": (r) => r.status === 200,
    "login has token": (r) => JSON.parse(r.body).token !== undefined,
  });
  errorRate.add(!loginSuccess);

  sleep(0.5);

  // Create a test user
  const createStart = Date.now();
  const userName = `loadtest_${__VU}_${Date.now()}`;
  const createRes = http.post(
    `${BASE_URL}/api/users`,
    JSON.stringify({
      name: userName,
      rateLimit: 10,
      dailyLimit: 100,
    }),
    { headers }
  );
  createUserDuration.add(Date.now() - createStart);

  const createSuccess = check(createRes, {
    "create user status 200 or 201": (r) =>
      r.status === 200 || r.status === 201,
  });
  errorRate.add(!createSuccess);

  sleep(0.5);

  // Get users list
  const listRes = http.get(`${BASE_URL}/api/users`, { headers });
  const listSuccess = check(listRes, {
    "list users status 200": (r) => r.status === 200,
  });
  errorRate.add(!listSuccess);

  sleep(0.5);

  // Get stats
  const statsRes = http.get(`${BASE_URL}/api/stats`, { headers });
  const statsSuccess = check(statsRes, {
    "stats status 200": (r) => r.status === 200,
  });
  errorRate.add(!statsSuccess);

  sleep(1);
}

export function teardown(data) {
  console.log("Admin API load test completed");
}
