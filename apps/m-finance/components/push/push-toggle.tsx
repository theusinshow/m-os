"use client";

import { useEffect, useState } from "react";
import { Button } from "@/components/ui/button";
import { useToast } from "@/components/ui/toast";

// VAPID public key must be a Uint8Array for PushManager.subscribe.
function urlBase64ToUint8Array(base64: string) {
  const padding = "=".repeat((4 - (base64.length % 4)) % 4);
  const normalized = (base64 + padding).replace(/-/g, "+").replace(/_/g, "/");
  const raw = atob(normalized);
  const output = new Uint8Array(raw.length);
  for (let i = 0; i < raw.length; i += 1) output[i] = raw.charCodeAt(i);
  return output;
}

type State = "loading" | "unsupported" | "off" | "on" | "busy";

export function PushToggle() {
  const { addToast } = useToast();
  const [state, setState] = useState<State>("loading");

  const vapidKey = process.env.NEXT_PUBLIC_VAPID_PUBLIC_KEY ?? "";

  useEffect(() => {
    let active = true;
    (async () => {
      if (
        typeof window === "undefined" ||
        !("serviceWorker" in navigator) ||
        !("PushManager" in window)
      ) {
        if (active) setState("unsupported");
        return;
      }
      try {
        const reg = await navigator.serviceWorker.ready;
        const sub = await reg.pushManager.getSubscription();
        if (active) setState(sub ? "on" : "off");
      } catch {
        if (active) setState("off");
      }
    })();
    return () => {
      active = false;
    };
  }, []);

  async function enable() {
    if (!vapidKey) {
      addToast("Push não configurado (VAPID ausente).", "error");
      return;
    }
    setState("busy");
    try {
      const permission = await Notification.requestPermission();
      if (permission !== "granted") {
        addToast("Permissão de notificação negada.", "error");
        setState("off");
        return;
      }

      const reg = await navigator.serviceWorker.ready;
      const sub = await reg.pushManager.subscribe({
        userVisibleOnly: true,
        applicationServerKey: urlBase64ToUint8Array(vapidKey),
      });

      const res = await fetch("/api/push/subscribe", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ ...sub.toJSON(), userAgent: navigator.userAgent }),
      });
      if (!res.ok) throw new Error("save failed");

      addToast("Notificações ativadas neste dispositivo.", "success");
      setState("on");
    } catch {
      addToast("Não foi possível ativar as notificações.", "error");
      setState("off");
    }
  }

  async function disable() {
    setState("busy");
    try {
      const reg = await navigator.serviceWorker.ready;
      const sub = await reg.pushManager.getSubscription();
      if (sub) {
        await fetch("/api/push/subscribe", {
          method: "DELETE",
          headers: { "content-type": "application/json" },
          body: JSON.stringify({ endpoint: sub.endpoint }),
        });
        await sub.unsubscribe();
      }
      addToast("Notificações desativadas neste dispositivo.", "success");
      setState("off");
    } catch {
      addToast("Não foi possível desativar.", "error");
      setState("on");
    }
  }

  async function sendTest() {
    try {
      const res = await fetch("/api/push/test", { method: "POST" });
      const data = (await res.json()) as { delivered?: number };
      addToast(
        data.delivered ? "Teste enviado, confira a notificação." : "Nenhum dispositivo recebeu.",
        data.delivered ? "success" : "error",
      );
    } catch {
      addToast("Falha ao enviar teste.", "error");
    }
  }

  if (state === "loading") {
    return <p className="text-sm text-text-muted">Verificando suporte a notificações…</p>;
  }

  if (state === "unsupported") {
    return (
      <p className="text-sm leading-6 text-text-muted">
        Este navegador não suporta notificações push. No iPhone, instale o app na tela inicial
        (Compartilhar → Adicionar à Tela de Início) e abra por lá.
      </p>
    );
  }

  return (
    <div className="space-y-3">
      <p className="text-sm leading-6 text-text-muted">
        Receba um aviso no celular antes de uma cobrança, mesmo com o app fechado.
      </p>
      <div className="flex flex-wrap gap-2">
        {state === "on" ? (
          <>
            <Button onClick={disable} type="button" variant="secondary">
              Desativar notificações
            </Button>
            <Button onClick={sendTest} type="button" variant="ghost">
              Enviar teste
            </Button>
          </>
        ) : (
          <Button disabled={state === "busy"} onClick={enable} type="button">
            {state === "busy" ? "Ativando…" : "Ativar notificações"}
          </Button>
        )}
      </div>
    </div>
  );
}
