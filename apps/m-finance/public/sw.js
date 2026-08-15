// M Finance service worker.
// Intentionally minimal: it only caches build-time static assets so the app is
// installable and static loads are fast. It never intercepts navigations, RSC
// payloads, server actions or auth responses — those must always hit the network.
const CACHE = "mfinance-static-v1";

self.addEventListener("install", () => {
  self.skipWaiting();
});

self.addEventListener("activate", (event) => {
  event.waitUntil(
    caches
      .keys()
      .then((keys) => Promise.all(keys.filter((k) => k !== CACHE).map((k) => caches.delete(k))))
      .then(() => self.clients.claim()),
  );
});

// Web Push: show the reminder even when the app is closed.
self.addEventListener("push", (event) => {
  let payload = {};
  try {
    payload = event.data ? event.data.json() : {};
  } catch {
    payload = { title: "M Finance", body: event.data ? event.data.text() : "" };
  }

  const title = payload.title || "M Finance";
  const options = {
    body: payload.body || "",
    icon: "/icon.svg",
    badge: "/icon.svg",
    tag: payload.tag,
    data: { url: payload.url || "/app/dashboard" },
    requireInteraction: false,
  };

  event.waitUntil(self.registration.showNotification(title, options));
});

// Focus an open tab (or open one) when the notification is tapped.
self.addEventListener("notificationclick", (event) => {
  event.notification.close();
  const targetUrl = (event.notification.data && event.notification.data.url) || "/app/dashboard";

  event.waitUntil(
    self.clients.matchAll({ type: "window", includeUncontrolled: true }).then((clients) => {
      for (const client of clients) {
        if ("focus" in client) {
          client.navigate(targetUrl);
          return client.focus();
        }
      }
      return self.clients.openWindow(targetUrl);
    }),
  );
});

self.addEventListener("fetch", (event) => {
  const { request } = event;
  if (request.method !== "GET") return;

  const url = new URL(request.url);
  if (url.origin !== self.location.origin) return;
  // Only immutable build output. Everything else (pages, data, actions) is live.
  if (!url.pathname.startsWith("/_next/static/")) return;

  event.respondWith(
    caches.open(CACHE).then(async (cache) => {
      const cached = await cache.match(request);
      if (cached) return cached;
      const response = await fetch(request);
      if (response.ok) cache.put(request, response.clone());
      return response;
    }),
  );
});
