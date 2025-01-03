import "./globals.css";

export const metadata = {
  title: "Anichain",
  description: "Your Tauri + Next.js + Tailwind CSS app",
};

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="en">
      <body className="antialiased">{children}</body>
    </html>
  );
}
