import { createBrowserRouter } from "react-router-dom";
import { AppLayout } from "@/components/layout/AppLayout";
import { DashboardPage } from "@/features/dashboard/DashboardPage";
import { ProjectsPage } from "@/features/projects/ProjectsPage";
import { HistoryPage } from "@/features/history/HistoryPage";
import { ReconstructionPage } from "@/features/reconstruction/ReconstructionPage";
import { ReportsPage } from "@/features/reports/ReportsPage";
import { SettingsPage } from "@/features/settings/SettingsPage";
import { ROUTES } from "./routes";

export const router = createBrowserRouter([
  {
    path: "/",
    element: <AppLayout />,
    children: [
      { index: true, element: <DashboardPage /> },
      { path: ROUTES.projects, element: <ProjectsPage /> },
      { path: ROUTES.history, element: <HistoryPage /> },
      { path: ROUTES.timeline, element: <ReconstructionPage /> },
      { path: ROUTES.reports, element: <ReportsPage /> },
      { path: ROUTES.settings, element: <SettingsPage /> },
    ],
  },
]);
