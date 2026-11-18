(ns viewfinder.horizon
  "The horizon from one place: for each whole degree of azimuth, the highest elevation angle the
   ground reaches, with the earth's curve taken off. Azimuth 0 is north and angles run clockwise.
   The walk samples the nearest cell. A horizon is one place, so the field builds them as a wave
   of places rather than a wave of rays."
  (:require [viewfinder.dem :as dem]))

(set! *warn-on-reflection* true)
(set! *unchecked-math* :warn-on-boxed)

(def azimuths 360)                    ; one sample per degree, the resolution the field stores
(def eye-m 2.0)                       ; the camera above the bare earth
(def max-ground-m 4123.0)             ; Kings Peak, so a ray can stop once no Utah ground can win
(def min-distance-m 400.0)            ; closer than this is the slope underfoot, not the ridge in the frame
(def max-distance-m 50000.0)          ; past this a ridge is haze, not a skyline
(def step-deg 0.4)                    ; each step along a ray subtends about this much
(def radius-m (/ (* 6371000.0 7.0) 6.0)) ; earth radius plus the usual refraction (about 13%)
(def missing Short/MIN_VALUE)         ; a ray that never touched ground; not a real angle

(defn- opt
  [opts k default]
  (if (contains? opts k) (get opts k) default))

(defn- walk-angle
  "The winning sample along one azimuth: elevation angle in degrees, and meters from the camera.
   [NaN 0] when the ray never touches ground. lat-m and lon-m are meters per degree at the
   observer. The walk starts at min-m so a berm beside the camera cannot hide the ridge. The grid
   fields are read here, once per ray: a primitive function cannot take the cell coordinates as
   arguments."
  [grid lat lon azimuth eye-z lat-m lon-m min-m max-m radius growth]
  (let [^shorts samples (:dem/samples grid)
        width (long (:dem/width grid))
        height (long (:dem/height grid))
        north (double (:dem/north grid))
        west (double (:dem/west grid))
        dx (double (:dem/dx grid))
        dy (double (:dem/dy grid))
        lat (double lat)
        lon (double lon)
        eye-z (double eye-z)
        lat-m (double lat-m)
        lon-m (double lon-m)
        min-m (double min-m)
        max-m (double max-m)
        radius (double radius)
        growth (double growth)
        az (Math/toRadians (double azimuth))
        north-u (Math/cos az)
        east-u (Math/sin az)
        gap (long dem/nodata)
        max-ground (double max-ground-m)]
    (loop [d min-m best ##-Inf best-d 0.0 hit false]
      (if (> d max-m)
        (if hit [best best-d] [Double/NaN 0.0])
        (let [drop (/ (* d d) (* 2.0 radius))
              ceiling (Math/toDegrees (Math/atan2 (- (- max-ground eye-z) drop) d))]
          (if (and hit (< ceiling best))
            [best best-d]
            (let [lat2 (+ lat (/ (* d north-u) lat-m))
                  lon2 (+ lon (/ (* d east-u) lon-m))
                  col (Math/round (/ (- lon2 west) dx))
                  row (Math/round (/ (- lat2 north) dy))
                  z (if (or (neg? col) (neg? row) (>= col width) (>= row height))
                      gap
                      (long (aget samples (int (+ (* row width) col)))))]
              (if (== z gap)
                (recur (* d growth) best best-d hit)
                (let [ang (Math/toDegrees (Math/atan2 (- (- (double z) eye-z) drop) d))]
                  (if (> ang best)
                    (recur (* d growth) ang d true)
                    (recur (* d growth) best best-d true)))))))))))

(defn- centi
  [degrees]
  (if (Double/isNaN (double degrees))
    (short missing)
    (short (Math/round (* 100.0 (double degrees))))))

(defn view
  "The horizon from lat lon. :horizon/angles is 360 centidegree elevations. :horizon/distances
   is meters to the sample that won each degree. Nil when that cell has no ground. A degree that
   never touches ground is the missing sentinel and a zero distance. opts may override :min-m,
   :max-m, :step-deg, :radius and :eye-m; the defaults are the Utah search."
  ([grid lat lon] (view grid lat lon {}))
  ([grid lat lon opts]
   (when-let [ground (dem/elev-at grid lat lon)]
     (let [eye (+ (double (opt opts :eye-m eye-m)) (double ground))
           lat-m 111320.0
           lon-m (* 111320.0 (Math/cos (Math/toRadians lat)))
           step (double (opt opts :step-deg step-deg))
           growth (+ 1.0 (Math/tan (Math/toRadians step)))
           min-m (double (opt opts :min-m min-distance-m))
           max-m (double (opt opts :max-m max-distance-m))
           radius (double (opt opts :radius radius-m))
           angles (short-array azimuths)
           distances (float-array azimuths)]
       (dotimes [az azimuths]
         (let [[ang dist] (walk-angle grid lat lon az eye lat-m lon-m min-m max-m radius growth)]
           (aset angles az (short (centi ang)))
           (aset distances az (float dist))))
       {:horizon/angles angles
        :horizon/distances distances}))))

(defn centidegrees
  "360 centidegree elevations from lat lon, or nil when that cell has no ground. A degree that
   never touches ground is the missing sentinel. opts are the opts of view."
  (^shorts [grid lat lon] (centidegrees grid lat lon {}))
  (^shorts [grid lat lon opts]
   (when-let [seen (view grid lat lon opts)]
     (:horizon/angles seen))))

(defn horizon
  "360 maps of :horizon/azimuth and :horizon/elevation (degrees) from lat lon. A degree that
   never touches ground has elevation nil. opts are the opts of centidegrees."
  ([grid lat lon] (horizon grid lat lon {}))
  ([grid lat lon opts]
   (when-let [angles (centidegrees grid lat lon opts)]
     (mapv (fn [azimuth]
             (let [c (aget ^shorts angles azimuth)]
               {:horizon/azimuth azimuth
                :horizon/elevation (when-not (== c (long missing)) (/ (double c) 100.0))}))
           (range azimuths)))))

(defn ground-at
  "Meters of bare earth under lat lon, or nil off the grid. The eye sits eye-m above this."
  [grid lat lon]
  (dem/elev-at grid lat lon))
