-- ============================================================================
-- Derovia — schema de base de donnees
-- ----------------------------------------------------------------------------
-- A coller entierement dans Supabase : menu de gauche « SQL Editor », puis
-- « New query », coller, et cliquer « Run ».
--
-- Ce script peut etre relance sans dommage : il ne cree que ce qui manque et ne
-- supprime jamais de donnees.
--
-- Deux tables seulement :
--   profils   — le nom affiche de chaque compte
--   scenarios — les arbitrages enregistres, ce que la fenetre de connexion
--               promet de retrouver « sur tous vos postes »
--
-- Aucun historique de conversion ni de compression n'est stocke : les noms de
-- fichiers partiraient sur un serveur, ce qui contredirait la promesse affichee
-- dans l'application.
-- ============================================================================


-- ---------------------------------------------------------------------------
-- 1. Les profils
-- ---------------------------------------------------------------------------
-- Un profil par compte. La cle primaire est l'identifiant du compte lui-meme :
-- un profil ne peut donc pas exister sans compte, ni en double.

create table if not exists public.profils (
  id         uuid primary key references auth.users (id) on delete cascade,
  nom        text,
  cree_le    timestamptz not null default now(),
  modifie_le timestamptz not null default now()
);

comment on table public.profils is
  'Le nom affiche de chaque compte Derovia.';


-- ---------------------------------------------------------------------------
-- 2. Les scenarios d'arbitrage
-- ---------------------------------------------------------------------------
-- Les hypotheses sont stockees en JSON : le moteur les fait evoluer d'une
-- version a l'autre, et une colonne par parametre imposerait une migration a
-- chaque ajout.

create table if not exists public.scenarios (
  id           uuid primary key default gen_random_uuid(),
  utilisateur  uuid not null references auth.users (id) on delete cascade,
  nom          text not null,
  preset       text not null,
  hypotheses   jsonb not null,
  cree_le      timestamptz not null default now(),
  modifie_le   timestamptz not null default now()
);

comment on table public.scenarios is
  'Les arbitrages enregistres, retrouvables depuis n''importe quel poste.';

-- Lister ses scenarios du plus recent au plus ancien est la requete la plus
-- frequente : cet index la rend immediate meme avec des milliers de lignes.
create index if not exists scenarios_par_utilisateur
  on public.scenarios (utilisateur, modifie_le desc);


-- ---------------------------------------------------------------------------
-- 3. Row Level Security
-- ---------------------------------------------------------------------------
-- INDISPENSABLE. La cle « anon » est publique : elle se trouve dans chaque
-- binaire distribue. Sans RLS, n'importe qui pourrait lire toute la base avec
-- cette cle. Avec RLS, chaque compte ne voit que ses propres lignes.

alter table public.profils   enable row level security;
alter table public.scenarios enable row level security;

-- Les politiques sont supprimees puis recreees, pour que le script reste
-- relancable sans erreur « already exists ».

drop policy if exists "profil visible par son proprietaire"     on public.profils;
drop policy if exists "profil cree par son proprietaire"        on public.profils;
drop policy if exists "profil modifie par son proprietaire"     on public.profils;

create policy "profil visible par son proprietaire"
  on public.profils for select
  using ((select auth.uid()) = id);

create policy "profil cree par son proprietaire"
  on public.profils for insert
  with check ((select auth.uid()) = id);

create policy "profil modifie par son proprietaire"
  on public.profils for update
  using ((select auth.uid()) = id)
  with check ((select auth.uid()) = id);


drop policy if exists "scenarios visibles par leur proprietaire"  on public.scenarios;
drop policy if exists "scenarios crees par leur proprietaire"     on public.scenarios;
drop policy if exists "scenarios modifies par leur proprietaire"  on public.scenarios;
drop policy if exists "scenarios supprimes par leur proprietaire" on public.scenarios;

create policy "scenarios visibles par leur proprietaire"
  on public.scenarios for select
  using ((select auth.uid()) = utilisateur);

create policy "scenarios crees par leur proprietaire"
  on public.scenarios for insert
  with check ((select auth.uid()) = utilisateur);

create policy "scenarios modifies par leur proprietaire"
  on public.scenarios for update
  using ((select auth.uid()) = utilisateur)
  with check ((select auth.uid()) = utilisateur);

create policy "scenarios supprimes par leur proprietaire"
  on public.scenarios for delete
  using ((select auth.uid()) = utilisateur);


-- ---------------------------------------------------------------------------
-- 4. Un profil cree automatiquement a l'inscription
-- ---------------------------------------------------------------------------
-- Sans cela, l'application devrait creer le profil elle-meme apres chaque
-- inscription — et oublier de le faire une seule fois laisserait un compte
-- sans profil.

create or replace function public.creer_profil_a_l_inscription()
returns trigger
language plpgsql
security definer
set search_path = ''
as $$
begin
  insert into public.profils (id, nom)
  values (
    new.id,
    -- Le nom saisi a l'inscription, sinon la partie gauche de l'adresse.
    coalesce(new.raw_user_meta_data ->> 'nom', split_part(new.email, '@', 1))
  )
  on conflict (id) do nothing;
  return new;
end;
$$;

drop trigger if exists creer_profil on auth.users;

create trigger creer_profil
  after insert on auth.users
  for each row
  execute function public.creer_profil_a_l_inscription();


-- ---------------------------------------------------------------------------
-- 5. La date de modification, tenue a jour toute seule
-- ---------------------------------------------------------------------------

create or replace function public.marquer_modification()
returns trigger
language plpgsql
as $$
begin
  new.modifie_le = now();
  return new;
end;
$$;

drop trigger if exists marquer_modification on public.profils;
create trigger marquer_modification
  before update on public.profils
  for each row execute function public.marquer_modification();

drop trigger if exists marquer_modification on public.scenarios;
create trigger marquer_modification
  before update on public.scenarios
  for each row execute function public.marquer_modification();


-- ---------------------------------------------------------------------------
-- 6. Verification
-- ---------------------------------------------------------------------------
-- Les deux lignes doivent afficher « true » dans la colonne rls_actif.
-- Si l'une affiche « false », le RLS n'est pas actif et la table est exposee.

select
  tablename                as nom_de_table,
  rowsecurity              as rls_actif,
  (select count(*) from pg_policies p
    where p.tablename = t.tablename)  as nombre_de_politiques
from pg_tables t
where schemaname = 'public'
  and tablename in ('profils', 'scenarios');
