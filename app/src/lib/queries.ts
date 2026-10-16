import { type QueryClient, useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useRef, useState } from "react";
import { toast } from "sonner";
import { api, errorMessage } from "./api";
import { type Change, commands, type Level, type Snapshot } from "./bindings";
import { withChange } from "./snapshot";

const SNAPSHOT = ["snapshot"] as const;

export function useSnapshot() {
  return useQuery({ queryKey: SNAPSHOT, queryFn: () => api.snapshot(), refetchInterval: 1000 });
}

/** Shows a change at once, sends it, and rolls back with a message if it fails. */
async function optimistic(client: QueryClient, update: (s: Snapshot) => Snapshot) {
  await client.cancelQueries({ queryKey: SNAPSHOT });
  const before = client.getQueryData<Snapshot>(SNAPSHOT);
  if (before) client.setQueryData(SNAPSHOT, update(before));
  return { before };
}

function useSnapshotMutation<V>(send: (v: V) => Promise<unknown>, update?: (s: Snapshot, v: V) => Snapshot) {
  const client = useQueryClient();
  return useMutation({
    mutationFn: send,
    onMutate: (v: V) => (update ? optimistic(client, (s) => update(s, v)) : Promise.resolve({ before: undefined })),
    onError: (error, _v, ctx) => {
      if (ctx?.before) client.setQueryData(SNAPSHOT, ctx.before);
      toast.error(errorMessage(error));
    },
    onSettled: () => client.invalidateQueries({ queryKey: SNAPSHOT }),
  });
}

export function useApply() {
  return useSnapshotMutation((c: Change) => api.applyChange(c), withChange);
}

export function useApplyEq() {
  return useSnapshotMutation((id: string) => api.applyEq(id));
}

export function useSetBoost() {
  return useSnapshotMutation(
    (db: number) => api.setBoost(db),
    (s, db) => ({ ...s, boostDb: db }),
  );
}

export function useSaveProfile() {
  return useSnapshotMutation((name: string) => api.saveProfile(name));
}

export function useRenameProfile() {
  return useSnapshotMutation(({ id, name }: { id: string; name: string }) => api.renameProfile(id, name));
}

export function useDeleteProfile() {
  return useSnapshotMutation((id: string) => api.deleteProfile(id));
}

export function useRestoreOnConnect() {
  return useSnapshotMutation(
    (on: boolean) => api.setRestoreOnConnect(on),
    (s, on) => ({ ...s, settings: { ...s.settings, restoreOnConnect: on } }),
  );
}

export function useApplyCalibration() {
  return useSnapshotMutation(({ gain, boostDb }: { gain: number; boostDb: number }) =>
    api.applyCalibration(gain, boostDb),
  );
}

const SILENT: Level = { currentDb: -60, peakDb: -60, clipping: false };
/** The live level, polled about 16 times a second while mounted. */
export function useLevel(intervalMs = 60): Level {
  const [level, setLevel] = useState<Level>(SILENT);
  useEffect(() => {
    let alive = true;
    const tick = async () => {
      try {
        const l = await commands.level();
        if (alive) setLevel(l);
      } catch {
        if (alive) setLevel(SILENT);
      }
    };
    void tick();
    const id = setInterval(tick, intervalMs);
    return () => {
      alive = false;
      clearInterval(id);
    };
  }, [intervalMs]);
  return level;
}

/**
 * A slider's value while it is dragged: shown at once, sent at most every
 * `ms`, and the last value always sent on release.
 */
export function useDraft(value: number, send: (v: number) => void, ms = 80) {
  const [draft, setDraft] = useState<number | null>(null);
  const last = useRef(0);
  const pending = useRef<ReturnType<typeof setTimeout> | null>(null);
  useEffect(() => () => void (pending.current && clearTimeout(pending.current)), []);
  const change = (v: number) => {
    setDraft(v);
    const now = Date.now();
    if (pending.current) clearTimeout(pending.current);
    if (now - last.current >= ms) {
      last.current = now;
      send(v);
    } else {
      pending.current = setTimeout(() => {
        last.current = Date.now();
        send(v);
      }, ms);
    }
  };
  const commit = (v: number) => {
    if (pending.current) clearTimeout(pending.current);
    send(v);
    setDraft(null);
  };
  return { value: draft ?? value, change, commit };
}
