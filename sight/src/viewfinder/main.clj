(ns viewfinder.main
  "Build a horizon field from an ENVI grid, or ask that field where a photograph was taken.
   The search is the field. Nothing in here names a town or a summit."
  (:require [clojure.pprint :as pp]
            [clojure.string :as string]
            [viewfinder.dem :as dem]
            [viewfinder.field :as field]
            [viewfinder.horizon :as horizon]
            [viewfinder.score :as score]
            [viewfinder.sky :as sky])
  (:import [java.awt BasicStroke Color Graphics2D RenderingHints]
           [java.awt.geom Path2D$Double]
           [java.awt.image BufferedImage]
           [java.io File]
           [javax.imageio ImageIO]))

(set! *warn-on-reflection* true)

(defn- info!
  "Bounds of the grid and the ground a hundredth of a degree in from each corner."
  [path]
  (let [g (dem/read-envi path)
        n (double (:dem/north g))
        w (double (:dem/west g))
        s (dem/south g)
        e (dem/east g)]
    (pp/pprint {:north n
                :south s
                :west w
                :east e
                :width (:dem/width g)
                :height (:dem/height g)
                :dx (:dem/dx g)
                :dy (:dem/dy g)
                :nw (dem/elev-at g (- n 0.01) (+ w 0.01))
                :ne (dem/elev-at g (- n 0.01) (- e 0.01))
                :sw (dem/elev-at g (+ s 0.01) (+ w 0.01))
                :se (dem/elev-at g (+ s 0.01) (- e 0.01))})))

(defn- time!
  "How long one horizon takes at lat lon, and the highest angle it found."
  [path lat lon]
  (let [g (dem/read-envi path)
        t0 (System/nanoTime)
        samples (horizon/horizon g lat lon)
        ms (/ (double (- (System/nanoTime) t0)) 1e6)
        hit (reduce (fn [best sample]
                      (let [ang (:horizon/elevation sample)]
                        (cond
                          (nil? ang) best
                          (nil? best) sample
                          (> ang (:horizon/elevation best)) sample
                          :else best)))
                    nil
                    samples)]
    (pp/pprint {:ms ms
                :ground (horizon/ground-at g lat lon)
                :hit (count (filter :horizon/elevation samples))
                :max-deg (:horizon/elevation hit)
                :max-azimuth (:horizon/azimuth hit)})))

(defn- trace!
  "The skyline, as a fraction of the frame, so a pole or a cloud is obvious. Each tenth of the
   width reports its highest and lowest ridge."
  [path]
  (let [image (sky/read-image path)
        line (sky/trace image)
        width (:sky/width image)
        height (:sky/height image)
        step (max 1 (quot (count line) 36))]
    (println {:width width :height height :points (count line)})
    (doseq [p (take-nth step line)]
      (println (format "%.4f %.4f" (/ (:sky/x p) (double width)) (/ (:sky/y p) (double height)))))
    (doseq [band (range 10)]
      (let [lo (* band 0.1)
            hi (* (inc band) 0.1)
            ys (into []
                     (keep (fn [p]
                             (let [x (/ (:sky/x p) (double width))]
                               (when (and (<= lo x) (< x hi))
                                 (/ (:sky/y p) (double height))))))
                     line)]
        (when (seq ys)
          (println {:band band
                    :n (count ys)
                    :high (apply min ys)
                    :low (apply max ys)}))))))

(defn- build!
  "The field for a step-m lattice. With no box, the lattice is the whole grid."
  [grid-path out step box]
  (let [grid (dem/read-envi grid-path)
        [south north west east] (if (seq box)
                                  box
                                  [(dem/south grid)
                                   (double (:dem/north grid))
                                   (double (:dem/west grid))
                                   (dem/east grid)])
        places (vec (field/lattice south north west east step))
        n (field/build grid places out)]
    (println {:wrote n :places (count places) :out out})))

(defn- rough?
  "Whether the ground within radius-m of lat lon spans at least relief-m from lowest to highest.
   A camera among such ground sees its skyline move by degrees within a few hundred meters, so
   the field there needs a finer lattice than the open valley."
  [grid lat lon radius-m relief-m]
  (let [lat (double lat)
        lon (double lon)
        lat-m 111320.0
        lon-m (* lat-m (Math/cos (Math/toRadians lat)))
        zs (keep identity
                 (for [r [0.0 (/ (double radius-m) 3.0) (* 2.0 (/ (double radius-m) 3.0)) (double radius-m)]
                       az (range 0 360 22.5)]
                   (let [t (Math/toRadians (double az))]
                     (dem/elev-at grid
                                  (+ lat (/ (* r (Math/cos t)) lat-m))
                                  (+ lon (/ (* r (Math/sin t)) lon-m))))))]
    (and (seq zs)
         (>= (- (double (apply max zs)) (double (apply min zs))) (double relief-m)))))

(defn- build-rough!
  "The field for a step-m lattice over the whole grid, keeping only places where the ground
   within radius-m spans relief-m or more."
  [grid-path out step radius relief]
  (let [grid (dem/read-envi grid-path)
        all (field/lattice (dem/south grid) (double (:dem/north grid))
                           (double (:dem/west grid)) (dem/east grid) step)
        places (vec (filter (fn [[lat lon]] (rough? grid lat lon radius relief)) all))
        n (field/build grid places out)]
    (println {:lattice (count all) :rough (count places) :wrote n :out out})))

(defn- write-surface
  "One JSON array of lat, lon, and loss. The page draws this as a heatmap."
  [path rows]
  (spit path
        (str "["
             (string/join ","
                                  (map (fn [row]
                                         (format "{\"lat\":%.5f,\"lon\":%.5f,\"loss\":%.4f}"
                                                 (:place/lat row)
                                                 (:place/lon row)
                                                 (:place/loss row)))
                                       rows))
             "]")))

(defn- surface!
  "Every place's best loss, written as JSON for the match page. grid-path is the elevation the
   foreground drop is read from."
  [grid-path field-path image-path out]
  (let [grid (dem/read-envi grid-path)
        field (field/read-field field-path)
        image (sky/read-image image-path)
        line (sky/trace image)
        sampled (sky/decimate line sky/columns)
        rows (score/surface field grid sampled (:sky/width image) (:sky/height image))]
    (write-surface out rows)
    (println {:skyline (count line) :sampled (count sampled) :places (count rows) :out out})
    (pp/pprint (take 5 rows))))

(defn- query!
  "The best places in the field for the photograph. grid-path is the elevation the foreground
   drop is read from."
  [grid-path field-path image-path]
  (let [grid (dem/read-envi grid-path)
        field (field/read-field field-path)
        image (sky/read-image image-path)
        line (sky/trace image)
        sampled (sky/decimate line sky/columns)
        hits (score/locate field grid sampled (:sky/width image) (:sky/height image))]
    (println {:width (:sky/width image)
              :height (:sky/height image)
              :skyline (count line)
              :sampled (count sampled)
              :places (:field/count field)})
    (pp/pprint hits)))

(defn- around
  "Places on a step-m grid within reach-m of lat lon, as [lat lon]."
  [lat lon step-m reach-m]
  (let [lat (double lat)
        lon (double lon)
        step-m (double step-m)
        lat-m 111320.0
        lon-m (* lat-m (Math/cos (Math/toRadians lat)))
        n (long (/ (double reach-m) step-m))]
    (for [i (range (- n) (inc n))
          j (range (- n) (inc n))]
      [(+ lat (/ (* (double i) step-m) lat-m))
       (+ lon (/ (* (double j) step-m) lon-m))])))

(defn- closer
  "The ground around the best top of rows, on a step-m grid out to reach-m, walked fresh and
   ranked. The stored lattice is two kilometers. A camera a few kilometers from a ridge moves
   that ridge by degrees inside one cell."
  [grid sampled width height rows top step-m reach-m]
  (let [places (into []
                     (comp (mapcat (fn [row]
                                     (around (:place/lat row) (:place/lon row) step-m reach-m)))
                           (distinct))
                     (take top rows))]
    (score/surface (field/gather grid places) grid sampled width height)))

(defn- ridge
  "The ground the skyline sits on: for every whole degree inside the camera's field of view,
   [lat lon] of the sample that won that degree. seen is horizon/view of the place. A degree
   that never touched ground is left out."
  [seen lat lon yaw fov]
  (let [^floats distances (:horizon/distances seen)
        lat (double lat)
        lon (double lon)
        lat-m 111320.0
        lon-m (* lat-m (Math/cos (Math/toRadians lat)))
        half (long (Math/ceil (/ (double fov) 2.0)))]
    (into []
          (keep (fn [step]
                  (let [az (mod (+ (long yaw) (long step)) 360)
                        d (double (aget distances (int az)))
                        t (Math/toRadians (double az))]
                    (when (pos? d)
                      [(+ lat (/ (* d (Math/cos t)) lat-m))
                       (+ lon (/ (* d (Math/sin t)) lon-m))]))))
          (range (- half) (inc half)))))

(defn- ridges!
  "The ridge of each camera in a text file of lines: id lat lon yaw fov. Writes one JSON object
   of id to [[lat lon] ...]."
  [grid-path in out]
  (let [grid (dem/read-envi grid-path)
        rows (keep (fn [line]
                     (let [[id & nums] (string/split (string/trim line) #"\s+")]
                       (when (= 4 (count nums))
                         (let [[lat lon yaw fov] (mapv parse-double nums)]
                           [id (ridge (horizon/view grid lat lon) lat lon yaw fov)]))))
                   (string/split-lines (slurp in)))]
    (spit out
          (str "{"
               (string/join ","
                            (map (fn [[id pts]]
                                   (str "\"" id "\":["
                                        (string/join ","
                                                     (map (fn [[a b]] (format "[%.5f,%.5f]" a b)) pts))
                                        "]"))
                                 rows))
               "}"))
    (println {:cameras (count rows) :out out})))

(defn- write-match
  "One JSON object: the winning place, its camera, and its horizon as pixels of the photograph.
   The page draws the horizon over the skyline."
  [path place outline width height]
  (spit path
        (str (format "{\"width\":%d,\"height\":%d,\"lat\":%.5f,\"lon\":%.5f,\"elev\":%.0f,\"yaw\":%.0f,\"fov\":%.0f,\"roll\":%.0f,\"pitch\":%.2f,\"rmse\":%.3f,\"corr\":%.3f,\"loss\":%.4f,\"horizon\":["
                     (long width)
                     (long height)
                     (:place/lat place)
                     (:place/lon place)
                     (:place/elev place)
                     (:place/yaw place)
                     (:place/fov place)
                     (:place/roll place)
                     (:place/pitch place)
                     (:place/rmse place)
                     (:place/corr place)
                     (:place/loss place))
             (string/join "," (map (fn [[x y]] (format "[%.1f,%.1f]" x y)) outline))
             "]}")))

(defn- stroke!
  "Draws points, [x y] pairs, as one line on g."
  [^Graphics2D g ^Color color points]
  (when (seq points)
    (let [path (Path2D$Double.)
          [x0 y0] (first points)]
      (.moveTo path (double x0) (double y0))
      (doseq [[x y] (rest points)]
        (.lineTo path (double x) (double y)))
      (.setColor g color)
      (.draw g path))))

(defn- write-overlay
  "The photograph with the traced skyline in orange and the winner's horizon in cyan, as a JPEG.
   A person checks the trace and the fit with this before trusting the coordinate."
  [image-path path skyline outline]
  (let [^BufferedImage source (ImageIO/read (File. ^String image-path))
        canvas (BufferedImage. (.getWidth source) (.getHeight source) BufferedImage/TYPE_INT_RGB)
        g (.createGraphics canvas)
        thick (float (max 2.0 (/ (.getWidth source) 500.0)))]
    (.drawImage g source 0 0 nil)
    (.setRenderingHint g RenderingHints/KEY_ANTIALIASING RenderingHints/VALUE_ANTIALIAS_ON)
    (.setStroke g (BasicStroke. thick))
    (stroke! g (Color. 255 176 32) (map (fn [p] [(:sky/x p) (:sky/y p)]) skyline))
    (stroke! g (Color. 62 224 224) outline)
    (.dispose g)
    (ImageIO/write canvas "jpg" (File. ^String path))))

(defn- read-skyline
  "Skyline pixels from a JSON file of [x, y] pairs, the shape the match pages read. A photograph
   whose sky the tracer cannot tell from its snow or haze brings its own line."
  [path]
  (mapv (fn [[_ x y]] {:sky/x (parse-double x) :sky/y (parse-double y)})
        (re-seq #"\[\s*(-?\d+\.?\d*)\s*,\s*(-?\d+\.?\d*)\s*\]" (slurp path))))

(defn- around!
  "Every place within a kilometre of lat lon, 125 m apart, scored against the photograph, best
   first. A check: does the ground near a known camera fit, and by how much does the winner beat
   it. skyline-in is a skyline file or -, fov a known lens or -."
  [grid-path image-path lat lon skyline-in fov]
  (binding [score/*fovs* (if (= fov "-")
                           score/*fovs*
                           (let [f (parse-double fov)] [(- f 1.5) f (+ f 1.5)]))]
    (let [grid (dem/read-envi grid-path)
          image (sky/read-image image-path)
          line (if (= skyline-in "-") (sky/trace image) (read-skyline skyline-in))
          sampled (sky/decimate line sky/columns)
          rows (closer grid sampled (:sky/width image) (:sky/height image)
                       [{:place/lat (parse-double lat) :place/lon (parse-double lon)}]
                       1 125.0 1000.0)]
      (pp/pprint (take 5 rows)))))

(declare search-with)

(defn- search!
  "Where the photograph was taken: the stored field first, then fresh horizons every 500 m
   around the best of it, then every 125 m around the best of those. Writes every scored place
   for the heatmap and the winner's horizon for the overlay. With overlay-out, also paints the
   skyline and that horizon on the photograph. With skyline-in, that line is the skyline and
   the photograph is not traced. With fov, the lens is known (from the camera's focal length)
   and only widths within a degree and a half of it are tried."
  [grid-path field-path image-path surface-out match-out overlay-out skyline-in fov]
  (binding [score/*fovs* (if fov
                           (let [f (parse-double fov)] [(- f 1.5) f (+ f 1.5)])
                           score/*fovs*)]
    (search-with grid-path field-path image-path surface-out match-out overlay-out skyline-in)))

(defn- search-with
  "search! once the lens list is bound."
  [grid-path field-path image-path surface-out match-out overlay-out skyline-in]
  (let [grid (dem/read-envi grid-path)
        fields (mapv field/read-field (string/split field-path #","))
        image (sky/read-image image-path)
        width (:sky/width image)
        height (:sky/height image)
        line (if skyline-in (read-skyline skyline-in) (sky/trace image))
        sampled (sky/decimate line sky/columns)
        coarse (vec (sort-by :place/loss
                             (mapcat (fn [field] (score/surface field grid sampled width height))
                                     fields)))
        near (closer grid sampled width height coarse 60 250.0 500.0)
        fine (closer grid sampled width height near 15 100.0 300.0)
        rows (vec (sort-by :place/loss (concat coarse near fine)))
        best (first rows)
        seen (horizon/view grid (:place/lat best) (:place/lon best))
        outline (score/outline seen best width height)]
    (write-surface surface-out rows)
    (write-match match-out best outline width height)
    (when overlay-out
      (write-overlay image-path overlay-out line outline))
    (println {:coarse (count coarse) :near (count near) :fine (count fine)
              :surface surface-out :match match-out})
    (println "coarse")
    (pp/pprint (take 5 coarse))
    (println "all")
    (pp/pprint (take 10 rows))))

(defn -main
  "info GRID | time GRID LAT LON | trace IMAGE | build GRID OUT STEP [SOUTH NORTH WEST EAST] | query GRID FIELD IMAGE | surface GRID FIELD IMAGE OUT | search GRID FIELD IMAGE SURFACE-OUT MATCH-OUT [OVERLAY-OUT [SKYLINE-IN|- [FOV]]]"
  [& args]
  (case (first args)
    "info" (info! (second args))
    "time" (time! (second args) (parse-double (nth args 2)) (parse-double (nth args 3)))
    "trace" (trace! (second args))
    "build" (build! (second args) (nth args 2) (parse-double (nth args 3)) (mapv parse-double (drop 4 args)))
    "query" (query! (second args) (nth args 2) (nth args 3))
    "surface" (surface! (second args) (nth args 2) (nth args 3) (nth args 4))
    "ridges" (ridges! (second args) (nth args 2) (nth args 3))
    "build-rough" (build-rough! (second args) (nth args 2) (parse-double (nth args 3))
                                (parse-double (nth args 4)) (parse-double (nth args 5)))
    "around" (around! (second args) (nth args 2) (nth args 3) (nth args 4) (nth args 5)
                      (nth args 6))
    "search" (search! (second args) (nth args 2) (nth args 3) (nth args 4) (nth args 5)
                      (nth args 6 nil)
                      (let [s (nth args 7 nil)] (when-not (= s "-") s))
                      (nth args 8 nil))
    (do (println "usage: info | time | trace | build | query | surface | search")
        (System/exit 1))))
