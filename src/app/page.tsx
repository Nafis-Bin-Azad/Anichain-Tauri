import styles from "./page.module.css";
import Form from "./form";
export default function Home() {
  return (
    <div className={styles.page}>
      <main className="flex min-h-screen flex-col items-center justify-between p-24">
        <Form />
      </main>
    </div>
  );
}
