"use client";

import { Geist, Geist_Mono } from "next/font/google";
import "./globals.css";
import Link from "next/link";
import { usePathname } from "next/navigation";
import styles from "./layout.module.css"; // Add a CSS module for styling

const geistSans = Geist({
  variable: "--font-geist-sans",
  subsets: ["latin"],
});

const geistMono = Geist_Mono({
  variable: "--font-geist-mono",
  subsets: ["latin"],
});

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  const pathname = usePathname();

  return (
    <html lang="en">
      <body
        className={`${geistSans.variable} ${geistMono.variable} antialiased`}
      >
        <div className={styles.container}>
          <main className={styles.main}>{children}</main>
          <nav className={styles.navbar}>
            <Link
              href="/devices"
              className={pathname === "/devices" ? styles.active : ""}
            >
              <span role="img" aria-label="Devices">🌐</span>
              <span>Devices</span>
            </Link>
            <Link
              href="/program"
              className={pathname === "/program" ? styles.active : ""}
            >
              <span role="img" aria-label="Program">🖥️</span>
              <span>Program</span>
            </Link>
          </nav>
        </div>
      </body>
    </html>
  );
}
