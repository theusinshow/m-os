import { redirect } from "next/navigation";
import { getCurrentUser } from "@/lib/auth/guard";

export default async function HomePage() {
  const user = await getCurrentUser();

  if (user) {
    redirect("/app/dashboard");
  }

  redirect("/login");
}
