// The /print route's component: hands the validated search to the print page.
import { useSearch } from "@tanstack/react-router";
import { PrintPage, type PrintSearch } from "@/print/PrintPage";

export function PrintRoute() {
  const search = useSearch({ from: "/print" }) as PrintSearch;
  return <PrintPage search={search} />;
}
