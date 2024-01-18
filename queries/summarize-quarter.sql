SELECT SUM(amount) total_sales
FROM sales
WHERE close_date > date_trunc('quarter', now())
