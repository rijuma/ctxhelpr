// Cloudflare Worker for sh.ctxhelpr.dev
// Serves the install script and tracks installs via Analytics Engine.

const SCRIPT_URL =
  "https://raw.githubusercontent.com/rijuma/ctxhelpr/main/scripts/install.sh"

export default {
  async fetch(request, env, ctx) {
    if (new URL(request.url).pathname !== "/") {
      return new Response("Not found", { status: 404 })
    }

    ctx.waitUntil(logInstall(request, env))

    const resp = await fetch(SCRIPT_URL, {
      cf: { cacheTtl: 300, cacheEverything: true },
    })

    if (!resp.ok) {
      return new Response("Failed to fetch install script", { status: 502 })
    }

    return new Response(await resp.text(), {
      headers: {
        "Content-Type": "text/plain; charset=utf-8",
        "Cache-Control": "public, max-age=300",
      },
    })
  },
}

function parseOS(ua) {
  if (!ua) return "unknown"
  if (ua.includes("Linux")) return "linux"
  if (ua.includes("Darwin") || ua.includes("Macintosh")) return "macos"
  if (ua.includes("Windows")) return "windows"
  return "unknown"
}

function isCliClient(ua) {
  return /^(curl|Wget|libcurl|fetch)\b/i.test(ua)
}

async function logInstall(request, env) {
  if (!env.INSTALLS) return

  const ua = request.headers.get("User-Agent") || ""
  if (!isCliClient(ua)) return

  const cf = request.cf || {}

  env.INSTALLS.writeDataPoint({
    indexes: [cf.country || "unknown"],
    blobs: [parseOS(ua), cf.region || "", cf.city || "", ua],
    doubles: [1],
  })
}
