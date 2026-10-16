import { type ReactNode, useState } from "react";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogDescription, DialogTitle, DialogTrigger } from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import type { Profile } from "@/lib/bindings";
import { useRenameProfile, useSaveProfile } from "@/lib/queries";
import { m } from "@/paraglide/messages";

/** Saves the current EQ under a name, or renames an own profile. */
export function ProfileDialog({ trigger, rename }: { trigger: ReactNode; rename?: Profile }) {
  const [open, setOpen] = useState(false);
  const [name, setName] = useState(rename?.name ?? "");
  const save = useSaveProfile();
  const ren = useRenameProfile();
  const submit = () => {
    const n = name.trim();
    if (!n) return;
    if (rename) ren.mutate({ id: rename.id, name: n });
    else save.mutate(n);
    setOpen(false);
    if (!rename) setName("");
  };
  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogTrigger asChild>{trigger}</DialogTrigger>
      <DialogContent>
        <DialogTitle>{rename ? m.profile_rename_title() : m.profile_new_title()}</DialogTitle>
        {!rename && <DialogDescription>{m.profile_new_text()}</DialogDescription>}
        <form
          className="flex flex-col gap-4"
          onSubmit={(e) => {
            e.preventDefault();
            submit();
          }}
        >
          <div className="flex flex-col gap-1.5">
            <label htmlFor="profile-name" className="font-medium">
              {m.profile_name()}
            </label>
            <Input
              id="profile-name"
              autoFocus
              value={name}
              placeholder={m.profile_name_placeholder()}
              onChange={(e) => setName(e.target.value)}
            />
          </div>
          <div className="flex justify-end gap-2">
            <Button variant="outline" onClick={() => setOpen(false)}>
              {m.common_cancel()}
            </Button>
            <Button type="submit" disabled={!name.trim()}>
              {m.common_save()}
            </Button>
          </div>
        </form>
      </DialogContent>
    </Dialog>
  );
}
