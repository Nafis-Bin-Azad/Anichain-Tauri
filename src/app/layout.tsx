import type { Metadata } from "next";
import { Inter } from "next/font/google";
import "./globals.css";
import { ToastProvider } from "@/contexts/ToastContext";
import Navbar from "@/components/Navbar";

const inter = Inter({ subsets: ["latin"] });

export const metadata: Metadata = {
  title: "AniChain",
  description: "Anime Management System",
};

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="en" className="h-full">
      <body className={`${inter.className} h-full bg-gray-50`}>
        <ToastProvider>
          <div className="min-h-screen">
            <Navbar />
            <main className="container mx-auto px-4 pt-20 pb-8">
              {children}
            </main>
          </div>
        </ToastProvider>
      </body>
    </html>
  );
}
